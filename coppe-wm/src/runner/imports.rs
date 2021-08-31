use log::*;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{HashMap, VecDeque},
    sync::{mpsc::SyncSender, Arc},
};
use wasmer::{
    imports, Array, Function, Global, ImportObject, LazyInit, Memory, Store, Val, WasmPtr,
    WasmerEnv,
};
use x11rb::protocol::xproto::ConfigureWindowAux;

use super::plug_mgr::PluginId;
use super::sub_mgr::SubscriptionManager;
use crate::events::{Command, EncodedEvent, Subscription};
use crate::state::State;

#[derive(WasmerEnv, Clone)]
struct ConnEnv {
    wm_state: State,
    conn: SyncSender<Command>,
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
    conn: SyncSender<Command>,
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<EncodedEvent>>>>>,
    state: State,
) -> ImportObject {
    let conn_env = ConnEnv {
        conn: conn.clone(),
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
            "debug_log" => Function::new_native_with_env(store, conn_env.clone(), debug_log),
            "move_window" => Function::new_native_with_env(store, conn_env.clone(), move_window),
            "spawn" => Function::new_native_with_env(store, conn_env.clone(), spawn),
        }
    }
}

/// Subscribe to a specific WM event.
///
/// Expects a pointer to a buffer, describing event and it's optional filters.
///
/// Event description buffer has the following format:
/// * `<event_id: dword>` - see [events::id](crate::events::id);
/// * `<event_payload: dword array>` - size and fields expected depend on `event_id`, see [EncodedEvent];
/// * `[event_filter: <event_filter_id: dword>, <event_filter_payload: dword array>]`;
fn subscribe(env: &SubEnv, event_ptr: WasmPtr<i32, Array>, event_len: u32) -> i32 {
    env.read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;
            let event = event_ptr
                .deref(memory, 0, event_len)
                .ok_or(ErrorCode::BadArgument)?;
            let event: Vec<i32> = event.into_iter().map(|cell| cell.get()).collect();
            let sub = Subscription::parse(event.as_ref()).ok_or(ErrorCode::BadArgument)?;
            info!("{}: subscribe to {:?}", id, sub);
            env.subscriptions.write().subscribe(id, sub);
            Ok(())
        })
        .err()
        .unwrap_or(ErrorCode::Ok) as i32
}

/// Unsubscribe from a specific WM event.
///
/// Expects a pointer to a buffer, describing event and it's optional filters. If no filters are
/// passed, subscription for all matching events will be cancelled.
///
/// Event description buffer has the following format:
/// * `<event_id: dword>` - see [events::id](crate::events::id);
/// * `<event_payload: dword array>` - size and fields expected depend on `event_id`, see [EncodedEvent];
/// * `[event_filter: <event_filter_id: dword>, <event_filter_payload: dword array>]`;
fn unsubscribe(env: &SubEnv, event_ptr: WasmPtr<i32, Array>, event_len: u32) -> i32 {
    env.read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;
            let event = event_ptr
                .deref(memory, 0, event_len)
                .ok_or(ErrorCode::BadArgument)?;
            let event: Vec<i32> = event.into_iter().map(|cell| cell.get()).collect();
            let sub = Subscription::parse(event.as_ref()).ok_or(ErrorCode::BadArgument)?;
            info!("{}: unsubscribe from {:?}", id, sub);
            env.subscriptions.write().unsubscribe(&id, &sub);
            Ok(())
        })
        .err()
        .unwrap_or(ErrorCode::Ok) as i32
}

/// Read the next event. Returns number of read dwords or -1 if plugin id is unknown or writing to buffer is impossible.
/// If dwords are read to end, event is removed from queue. This will happen even if first dwords were never read.
fn event_read(env: &EventEnv, buf_ptr: WasmPtr<i32, Array>, buf_len: u32, read_offset: u32) -> i32 {
    let res = env
        .read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;

            let events = env.events.read();
            let plugin_events = if let Some(e) = events.get(&id) {
                e
            } else {
                return Ok(0);
            };
            let mut plugin_events = plugin_events.lock();

            let event = if let Some(e) = plugin_events.front() {
                e
            } else {
                return Ok(0);
            };

            let read_len = std::cmp::min(buf_len as i32, event.size() as i32 - read_offset as i32);
            // Return error if offset is greater than event length
            if read_len < 0 {
                return Err(ErrorCode::BadArgument);
            }

            unsafe {
                let ptr = buf_ptr
                    .deref_mut(memory, 0, buf_len)
                    .ok_or(ErrorCode::BadArgument)?;

                for i in 0..read_len as usize {
                    ptr[i].set(event[read_offset as usize + i]);
                }
            }
            info!(
                "{}: event_read; Response: {:?}, {} dwords",
                id, event, read_len
            );

            // Pop event if it was read to end
            if event.size() - read_offset as usize == read_len as usize {
                plugin_events.pop_front();
            }

            Ok(read_len)
        });

    match res {
        Ok(v) => v as i32,
        Err(v) => v as i32,
    }
}

/// Query the size of next event.
///
/// Returns size of next event or 0 if no event is in queue or error code.
fn event_size(env: &EventEnv) -> i32 {
    let res = env.read_id().ok_or(ErrorCode::UnableToGetId).map(|id| {
        let events = env.events.read();
        events
            .get(&id)
            .map(|plugin_events| {
                let plugin_events = plugin_events.lock();

                let size = plugin_events.front().map(|event| event.size()).unwrap_or(0);
                info!("{}: event_size; Response: {}", id, size);
                size as i32
            })
            .unwrap_or(0)
    });
    match res {
        Ok(v) => v as i32,
        Err(v) => v as i32,
    }
}

fn debug_log(env: &ConnEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) -> i32 {
    env.read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;
            let debug_string = unsafe {
                cmd_ptr
                    .get_utf8_str(memory, cmd_len)
                    .ok_or(ErrorCode::BadArgument)?
            };
            info!("{}: {}", id, debug_string);
            Ok(())
        })
        .err()
        .unwrap_or(ErrorCode::Ok) as i32
}

fn move_window(env: &ConnEnv, window_id: u32, x: i32, y: i32) -> i32 {
    env.read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            info!("{}: move_window {} to [{}, {}]", id, window_id, x, y);
            let aux = ConfigureWindowAux::default().x(x).y(y);

            env.conn
                .send(Command::ConfigureWindow(aux))
                .map_err(|_| ErrorCode::Send)
        })
        .err()
        .unwrap_or(ErrorCode::Ok) as i32
}

fn spawn(env: &ConnEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) -> i32 {
    env.read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;
            let cmd_string = cmd_ptr
                .get_utf8_string(memory, cmd_len)
                .ok_or(ErrorCode::BadArgument)?;
            info!("{}: spawn '{}'", id, cmd_string);
            shlex::split(&cmd_string).ok_or(ErrorCode::BadArgument)
        })
        .and_then(|cmd_args| {
            (cmd_args.len() > 0)
                .then(|| cmd_args)
                .ok_or(ErrorCode::BadArgument)
        })
        .and_then(|cmd_args| {
            std::process::Command::new(&cmd_args[0])
                .args(&cmd_args[1..])
                .spawn()
                .map_err(|_| ErrorCode::Execution)
        })
        .err()
        .unwrap_or(ErrorCode::Ok) as i32
}

enum ErrorCode {
    UnableToGetId = -128,
    UnableToGetMemory = -127,
    Send = -126,
    BadArgument = -2,
    Execution = -1,
    Ok = 0,
}
