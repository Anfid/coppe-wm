use log::*;
use std::sync::{Arc, Mutex};
use wasmer::{
    imports, Array, Function, Global, ImportObject, LazyInit, Memory, Store, Val, WasmPtr,
    WasmerEnv,
};
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt};

use super::sub_mgr::{EventSubscription, SubscriptionManager};
use crate::state::State;
use crate::X11Conn;

#[derive(WasmerEnv, Clone)]
struct ConnEnv {
    wm_state: State,
    conn: Arc<X11Conn>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    #[wasmer(export)]
    id: LazyInit<Global>,
}

#[derive(WasmerEnv, Clone)]
struct SubEnv {
    subscriptions: Arc<Mutex<SubscriptionManager>>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    #[wasmer(export)]
    id: LazyInit<Global>,
}

fn read_id(id: Option<&Global>, mem: &Memory) -> Option<String> {
    id.map(|g| g.get())
        .and_then(|val| {
            if let Val::I32(val) = val {
                Some(val as u32)
            } else {
                None
            }
        })
        .map(|offset| WasmPtr::new(offset))
        .and_then(|ptr: WasmPtr<u8, Array>| ptr.get_utf8_string_with_nul(mem))
}

impl ConnEnv {
    fn read_id(&self) -> Option<String> {
        read_id(self.id_ref(), self.memory_ref().unwrap())
    }
}

impl SubEnv {
    fn read_id(&self) -> Option<String> {
        read_id(self.id_ref(), self.memory_ref().unwrap())
    }
}

pub(super) fn import_objects(
    store: &Store,
    conn: Arc<X11Conn>,
    subscriptions: Arc<Mutex<SubscriptionManager>>,
    state: State,
) -> ImportObject {
    let conn_env = ConnEnv {
        conn,
        wm_state: state,
        memory: Default::default(),
        id: Default::default(),
    };
    let sub_env = SubEnv {
        subscriptions,
        memory: Default::default(),
        id: Default::default(),
    };

    imports! {
        "env" => {
            "subscribe" => Function::new_native_with_env(store, sub_env.clone(), subscribe),
            "unsubscribe" => Function::new_native_with_env(store, sub_env.clone(), unsubscribe),
            "move_window" => Function::new_native_with_env(store, conn_env.clone(), move_window),
            "spawn" => Function::new_native_with_env(store, conn_env.clone(), spawn),
        }
    }
}

fn subscribe(env: &SubEnv, event_id: u32) {
    info!("Trying to sub to {}", event_id);
    env.read_id()
        .and_then(|p| EventSubscription::try_from(event_id).map(|e| (p, e)))
        .map(|(p, e)| {
            info!("subscribe to '{:?}' by {}", e, p);
            env.subscriptions.lock().unwrap().subscribe(p, e)
        });
}

fn unsubscribe(env: &SubEnv, event_id: u32) {
    env.read_id()
        .and_then(|p| EventSubscription::try_from(event_id).map(|e| (p, e)))
        .map(|(p, e)| {
            info!("unsubscribe from '{:?}' by {}", e, p);
            env.subscriptions.lock().unwrap().unsubscribe(&p, &e)
        });
}

fn move_window(env: &ConnEnv, id: u32, x: i32, y: i32) {
    let plugin_id = env.read_id();
    info!("move_window {} to [{}, {}] by {:?}", id, x, y, plugin_id);

    let aux = ConfigureWindowAux::default().x(x).y(y);

    env.conn.configure_window(id, &aux).unwrap();
}

fn spawn(env: &ConnEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) {
    env.memory_ref()
        .and_then(|memory| {
            let id = env.read_id()?;
            let cmd_string = cmd_ptr.get_utf8_string(memory, cmd_len)?;
            info!("spawn '{}' by {}", cmd_string, id);
            Some(cmd_string)
        })
        .and_then(|cmd_string| shlex::split(&cmd_string))
        .filter(|cmd_args| cmd_args.len() > 0)
        .map(|cmd_args| {
            std::process::Command::new(&cmd_args[0])
                .args(&cmd_args[1..])
                .spawn()
        });
}
