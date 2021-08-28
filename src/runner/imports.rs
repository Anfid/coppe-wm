use log::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, RwLock},
};
use wasmer::{
    imports, Array, Function, Global, ImportObject, LazyInit, Memory, Store, Val, WasmPtr,
    WasmerEnv,
};
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt};

use super::plug_mgr::PluginId;
use super::sub_mgr::{EventSubscription, SubscriptionManager};
use crate::events::EncodedEvent;
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
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    #[wasmer(export)]
    id: LazyInit<Global>,
}

#[derive(WasmerEnv, Clone)]
struct EventEnv {
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<EncodedEvent>>>>>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    #[wasmer(export)]
    id: LazyInit<Global>,
}

fn read_id(id: Option<&Global>, mem: &Memory) -> Option<PluginId> {
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
        .map(Into::into)
}

impl ConnEnv {
    fn read_id(&self) -> Option<PluginId> {
        read_id(self.id_ref(), self.memory_ref().unwrap())
    }
}

impl SubEnv {
    fn read_id(&self) -> Option<PluginId> {
        read_id(self.id_ref(), self.memory_ref().unwrap())
    }
}

impl EventEnv {
    fn read_id(&self) -> Option<PluginId> {
        read_id(self.id_ref(), self.memory_ref().unwrap())
    }
}

pub(super) fn import_objects(
    store: &Store,
    conn: Arc<X11Conn>,
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<EncodedEvent>>>>>,
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
    let event_env = EventEnv {
        events,
        memory: Default::default(),
        id: Default::default(),
    };

    imports! {
        "env" => {
            "subscribe" => Function::new_native_with_env(store, sub_env.clone(), subscribe),
            "unsubscribe" => Function::new_native_with_env(store, sub_env.clone(), unsubscribe),
            "event_read" => Function::new_native_with_env(store, event_env.clone(), event_read),
            "event_size" => Function::new_native_with_env(store, event_env.clone(), event_size),
            "debug" => Function::new_native_with_env(store, conn_env.clone(), debug),
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
            env.subscriptions.write().unwrap().subscribe(p, e)
        });
}

fn unsubscribe(env: &SubEnv, event_id: u32) {
    env.read_id()
        .and_then(|p| EventSubscription::try_from(event_id).map(|e| (p, e)))
        .map(|(p, e)| {
            info!("unsubscribe from '{:?}' by {}", e, p);
            env.subscriptions.write().unwrap().unsubscribe(&p, &e)
        });
}

/// Read the next event. Returns number of read bytes or -1 if plugin id is unknown or writing to buffer is impossible.
/// If bytes are read to end, event is removed from queue. This will happen even if first bytes were never read.
fn event_read(env: &EventEnv, buf_ptr: WasmPtr<i32, Array>, buf_len: u32, read_offset: u32) -> i32 {
    env.read_id()
        .and_then(|plug_id| {
            let memory = env.memory_ref()?;

            let events = env.events.read().unwrap();
            // TODO REMOVE
            if let None = events.get(&plug_id) {
                warn!("NO EVENTS");
            }
            let mut events = events.get(&plug_id)?.lock().unwrap();
            let event = if let Some(e) = events.front() {
                e
            } else {
                return Some(0);
            };

            let read_len = std::cmp::min(buf_len as i32, event.size() as i32 - read_offset as i32);
            // TODO REMOVE
            warn!("read_len: {}", read_len);
            // Return error if offset is greater than event length
            if read_len < 0 {
                return None;
            }

            unsafe {
                let ptr = buf_ptr.deref_mut(memory, 0, buf_len)?;

                for i in 0..read_len as usize {
                    ptr[i].set(event[read_offset as usize + i]);
                }
            }
            info!("event_read by {}: {:?}, {}", plug_id, event, read_len);

            // Pop event if it was read to end
            if event.size() - read_offset as usize == read_len as usize {
                events.pop_front();
            }

            Some(read_len)
        })
        .unwrap_or(-1)
}

/// Query the size of next event. Returns 0 if no event is in queue or -1 if plugin id is unknown.
fn event_size(env: &EventEnv) -> i32 {
    env.read_id()
        .and_then(|plug_id| {
            let events = env.events.read().unwrap();
            let events = events.get(&plug_id)?.lock().unwrap();

            let size = events.front().map(|event| event.size()).unwrap_or(0);
            info!("event_size by {}: {}", plug_id, size);
            Some(size as i32)
        })
        .unwrap_or(-1)
}

fn debug(env: &ConnEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) {
    env.read_id().and_then(|plug_id| {
        let memory = env.memory_ref()?;
        let debug_string = unsafe { cmd_ptr.get_utf8_str(memory, cmd_len)? };
        info!("{}: {}", plug_id, debug_string);
        Some(())
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
            shlex::split(&cmd_string)
        })
        .filter(|cmd_args| cmd_args.len() > 0)
        .map(|cmd_args| {
            std::process::Command::new(&cmd_args[0])
                .args(&cmd_args[1..])
                .spawn()
        });
}
