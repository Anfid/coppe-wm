use coppe_common::{
    encoding::{Decode, Encode, EncodeExt},
    event::Event,
};
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
use crate::events::{Command, Subscription};
use crate::state::State;

#[derive(WasmerEnv, Clone)]
struct StateEnv {
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
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<Event>>>>>,
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

impl StateEnv {
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
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<Event>>>>>,
    state: State,
) -> ImportObject {
    let state_env = StateEnv {
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
            "event_len" => Function::new_native_with_env(store, event_env.clone(), event_len),
            "clients_read" => Function::new_native_with_env(store, state_env.clone(), clients_read),
            "clients_size" => Function::new_native_with_env(store, state_env.clone(), clients_size),
            "debug_log" => Function::new_native_with_env(store, state_env.clone(), debug_log),
            "move_window" => Function::new_native_with_env(store, state_env.clone(), move_window),
            "spawn" => Function::new_native_with_env(store, state_env.clone(), spawn),
        }
    }
}

/// Read byte slice to WASM buffer helper
unsafe fn write_to_ptr(
    data: &[u8],
    memory: &Memory,
    buf_ptr: WasmPtr<u8, Array>,
    buf_len: u32,
    read_offset: u32,
) -> Result<usize, ErrorCode> {
    let read_len = std::cmp::min(buf_len as i32, data.len() as i32 - read_offset as i32);
    // Return error if offset is greater than event length
    if read_len < 0 {
        return Err(ErrorCode::BadArgument);
    }

    let ptr = buf_ptr
        .deref_mut(memory, 0, buf_len)
        .ok_or(ErrorCode::BadArgument)?;

    for i in 0..read_len as usize {
        ptr[i].set(data[i]);
    }

    Ok(read_len as usize)
}

/// Subscribe to a specific WM event.
///
/// Expects a pointer to a buffer, describing event and it's optional filters.
///
/// Event description buffer has the following format:
/// * `<event_id: dword>` - see [events::id](crate::events::id);
/// * `<event_payload: dword array>` - size and fields expected depend on `event_id`, see [EncodedEvent];
/// * `[event_filter: <event_filter_id: dword>, <event_filter_payload: dword array>]`;
fn subscribe(env: &SubEnv, event_ptr: WasmPtr<u8, Array>, event_len: u32) -> i32 {
    env.read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;
            let event = event_ptr
                .deref(memory, 0, event_len)
                .ok_or(ErrorCode::BadArgument)?;
            let event: Vec<u8> = event.into_iter().map(|cell| cell.get()).collect();
            let sub = Subscription::decode(event.as_ref()).map_err(|_| ErrorCode::BadArgument)?;
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
fn unsubscribe(env: &SubEnv, event_ptr: WasmPtr<u8, Array>, event_len: u32) -> i32 {
    env.read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;
            let event = event_ptr
                .deref(memory, 0, event_len)
                .ok_or(ErrorCode::BadArgument)?;
            let event: Vec<u8> = event.into_iter().map(|cell| cell.get()).collect();
            let sub = Subscription::decode(event.as_ref()).map_err(|_| ErrorCode::BadArgument)?;
            info!("{}: unsubscribe from {:?}", id, sub);
            env.subscriptions.write().unsubscribe(&id, &sub);
            Ok(())
        })
        .err()
        .unwrap_or(ErrorCode::Ok) as i32
}

/// Read the next event. Returns number of read dwords or -1 if plugin id is unknown or writing to buffer is impossible.
/// If dwords are read to end, event is removed from queue. This will happen even if first dwords were never read.
fn event_read(env: &EventEnv, buf_ptr: WasmPtr<u8, Array>, buf_len: u32, read_offset: u32) -> i32 {
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

            let encoded_event = event.encode_to_vec().unwrap();

            let read_len =
                unsafe { write_to_ptr(&encoded_event, memory, buf_ptr, buf_len, read_offset)? };

            info!(
                "{}: event_read; Response: {:?}, {} dwords",
                id, event, read_len
            );

            // Pop event if it was read to end
            if encoded_event.len() - read_offset as usize == read_len as usize {
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
fn event_len(env: &EventEnv) -> i32 {
    let res = env.read_id().ok_or(ErrorCode::UnableToGetId).map(|id| {
        let events = env.events.read();
        events
            .get(&id)
            .map(|plugin_events| {
                let plugin_events = plugin_events.lock();

                let size = plugin_events
                    .front()
                    .map(|event| event.encoded_size())
                    .unwrap_or(0);
                info!("{}: event_len; Response: {}", id, size);
                size as i32
            })
            .unwrap_or(0)
    });

    match res {
        Ok(v) => v as i32,
        Err(v) => v as i32,
    }
}

/// Print debug message to logs. Returns 0 on success and error code on failure.
fn debug_log(env: &StateEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) -> i32 {
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

/// Read current client info. Since changes to clients can be made by any plugin, it is not guranteed to be persistent
/// between subsequent reads.
fn clients_read(env: &StateEnv, buf_ptr: WasmPtr<u8, Array>, buf_len: u32) -> i32 {
    let res = env
        .read_id()
        .ok_or(ErrorCode::UnableToGetId)
        .and_then(|id| {
            let memory = env.memory_ref().ok_or(ErrorCode::UnableToGetMemory)?;

            let state = env.wm_state.get();
            let buffer: Vec<u8> = state
                .clients
                .iter()
                .map(|c| c.encode_to_vec().unwrap())
                .flatten()
                .collect();

            let read_len = unsafe { write_to_ptr(buffer.as_ref(), memory, buf_ptr, buf_len, 0) }?;

            info!(
                "{}: clients_read; Response: {:?}, {} bytes",
                id,
                &buffer[..read_len],
                read_len
            );

            Ok(read_len)
        });

    match res {
        Ok(v) => v as i32,
        Err(v) => v as i32,
    }
}

/// Client info size hint. Since changes to clients can be made by any plugin, it is not guranteed to be persistent
/// between subsequent reads.
///
/// FIXME: this must be consistent for next reads. Potential fix is global state lock.
fn clients_size(env: &StateEnv) -> i32 {
    let res = env.read_id().ok_or(ErrorCode::UnableToGetId).map(|id| {
        let state = env.wm_state.get();
        let size: usize = state.clients.iter().map(|c| c.encoded_size()).sum();

        info!("{}: clients_size; Response: {} bytes", id, size);

        size
    });

    match res {
        Ok(v) => v as i32,
        Err(v) => v as i32,
    }
}

fn move_window(env: &StateEnv, window_id: u32, x: i32, y: i32) -> i32 {
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

fn spawn(env: &StateEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) -> i32 {
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

/// Core API call error codes.
#[derive(Debug)]
enum ErrorCode {
    /// Plugin could not be identified.
    UnableToGetId = -128,
    /// Plugin memory could not be accessed
    UnableToGetMemory = -127,
    /// Internal command send error.
    Send = -126,
    /// Invalid argument provided.
    BadArgument = -2,
    /// Command execution error.
    Execution = -1,
    /// Success.
    Ok = 0,
}
