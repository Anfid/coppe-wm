use coppe_common::{
    encoding::{Decode, Encode, EncodeExt},
    event::Event,
};
use log::*;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use wasmer::{imports, Array, Function, ImportObject, LazyInit, Memory, Store, WasmPtr, WasmerEnv};
use x11rb::errors::{ConnectionError as X11ConnectionError, ReplyError as X11ReplyError};

mod window;

use super::plug_mgr::PluginId;
use super::sub_mgr::SubscriptionManager;
use crate::events::Subscription;
use crate::x11::X11Info;

#[derive(WasmerEnv, Clone)]
struct XEnv {
    id: PluginId,
    x11: X11Info,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

#[derive(WasmerEnv, Clone)]
struct SubEnv {
    id: PluginId,
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

#[derive(WasmerEnv, Clone)]
struct EventEnv {
    id: PluginId,
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<Event>>>>>,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

pub(super) fn import_objects(
    plugin_id: PluginId,
    store: &Store,
    x11: X11Info,
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<Event>>>>>,
) -> ImportObject {
    let cmd_env = XEnv {
        id: plugin_id.clone(),
        x11: x11.clone(),
        memory: Default::default(),
    };
    let sub_env = SubEnv {
        id: plugin_id.clone(),
        subscriptions,
        memory: Default::default(),
    };
    let event_env = EventEnv {
        id: plugin_id.clone(),
        events,
        memory: Default::default(),
    };

    imports! {
        "env" => {
            "subscribe" => Function::new_native_with_env(store, sub_env.clone(), subscribe),
            "unsubscribe" => Function::new_native_with_env(store, sub_env.clone(), unsubscribe),
            "event_read" => Function::new_native_with_env(store, event_env.clone(), event_read),
            "event_len" => Function::new_native_with_env(store, event_env.clone(), event_len),
            "debug_log" => Function::new_native_with_env(store, cmd_env.clone(), debug_log),
            "window_move" => Function::new_native_with_env(store, cmd_env.clone(), window::window_move),
            "window_resize" => Function::new_native_with_env(store, cmd_env.clone(), window::window_resize),
            "window_move_resize" => Function::new_native_with_env(store, cmd_env.clone(), window::window_move_resize),
            "window_focus" => Function::new_native_with_env(store, cmd_env.clone(), window::window_focus),
            "window_get_properties" => Function::new_native_with_env(store, cmd_env.clone(), window::window_get_properties),
            "window_close" => Function::new_native_with_env(store, cmd_env.clone(), window::window_close),
            "spawn" => Function::new_native_with_env(store, cmd_env.clone(), spawn),
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
/// * `<event_id: [byte; 4]>` - see [events::id](crate::events::id);
/// * `<event_payload: byte array>` - size and contents depend on `event_id`, see [EncodedEvent];
/// * `[event_filter: <event_filter_id: [byte; 4]>, <event_filter_payload: byte array>]`;
fn subscribe(env: &SubEnv, event_ptr: WasmPtr<u8, Array>, event_len: u32) -> i32 {
    env.memory_ref()
        .ok_or(ErrorCode::UnableToGetMemory)
        .and_then(|memory| {
            let event = event_ptr
                .deref(memory, 0, event_len)
                .ok_or(ErrorCode::BadArgument)?;
            let event: Vec<u8> = event.into_iter().map(|cell| cell.get()).collect();
            let sub = Subscription::decode(event.as_ref()).map_err(|_| ErrorCode::BadArgument)?;
            info!("{}: subscribe to {:?}", env.id, sub);
            env.subscriptions.write().subscribe(env.id.clone(), sub);
            Ok(())
        })
        .value_or_error_code()
}

/// Unsubscribe from a specific WM event.
///
/// Expects a pointer to a buffer, describing event and it's optional filters. If no filters are
/// passed, subscription for all matching events will be cancelled. Returns 0 on success or error code.
///
/// Event description buffer has the following format:
/// * `<event_id: [byte; 4]>` - see [events::id](crate::events::id);
/// * `<event_payload: byte array>` - size and fields expected depend on `event_id`, see [EncodedEvent];
/// * `[event_filter: <event_filter_id: [byte; 4]>, <event_filter_payload: byte array>]`;
fn unsubscribe(env: &SubEnv, event_ptr: WasmPtr<u8, Array>, event_len: u32) -> i32 {
    env.memory_ref()
        .ok_or(ErrorCode::UnableToGetMemory)
        .and_then(|memory| {
            let event = event_ptr
                .deref(memory, 0, event_len)
                .ok_or(ErrorCode::BadArgument)?;
            let event: Vec<u8> = event.into_iter().map(|cell| cell.get()).collect();
            let sub = Subscription::decode(event.as_ref()).map_err(|_| ErrorCode::BadArgument)?;
            info!("{}: unsubscribe from {:?}", env.id, sub);
            env.subscriptions.write().unsubscribe(&env.id, &sub);
            Ok(())
        })
        .value_or_error_code()
}

/// Read the next event. Returns number of read bytes or -1 if plugin id is unknown or writing to buffer is impossible.
/// If bytes are read to end, event is removed from queue. This will happen even if first bytes were never read.
fn event_read(env: &EventEnv, buf_ptr: WasmPtr<u8, Array>, buf_len: u32, read_offset: u32) -> i32 {
    env.memory_ref()
        .ok_or(ErrorCode::UnableToGetMemory)
        .and_then(|memory| {
            let events = env.events.read();
            let plugin_events = if let Some(e) = events.get(&env.id) {
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
                env.id, event, read_len
            );

            // Pop event if it was read to end
            if encoded_event.len() - read_offset as usize == read_len as usize {
                plugin_events.pop_front();
            }

            Ok(read_len as u32)
        })
        .value_or_error_code()
}

/// Query the size of next event.
///
/// Returns size of next event in bytes or 0 if no event is in queue.
fn event_len(env: &EventEnv) -> u32 {
    let events = env.events.read();
    events
        .get(&env.id)
        .map(|plugin_events| {
            let plugin_events = plugin_events.lock();

            let size = plugin_events
                .front()
                .map(|event| event.encoded_size())
                .unwrap_or(0);
            info!("{}: event_len; Response: {}", env.id, size);
            size as u32
        })
        .unwrap_or(0)
}

/// Print debug message to logs. Returns 0 on success or error code.
fn debug_log(env: &XEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) -> i32 {
    env.memory_ref()
        .ok_or(ErrorCode::UnableToGetMemory)
        .and_then(|memory| {
            let debug_string = unsafe {
                cmd_ptr
                    .get_utf8_str(memory, cmd_len)
                    .ok_or(ErrorCode::BadArgument)?
            };
            info!("{}: {}", env.id, debug_string);
            Ok(())
        })
        .value_or_error_code()
}

fn spawn(env: &XEnv, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) -> i32 {
    env.memory_ref()
        .ok_or(ErrorCode::UnableToGetMemory)
        .and_then(|memory| {
            let cmd_string = cmd_ptr
                .get_utf8_string(memory, cmd_len)
                .ok_or(ErrorCode::BadArgument)?;
            info!("{}: spawn '{}'", env.id, cmd_string);
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
                .map(|_| {})
                .map_err(|_| ErrorCode::Execution)
        })
        .value_or_error_code()
}

/// Core API call error codes.
#[derive(Debug)]
enum ErrorCode {
    /// Plugin memory could not be accessed. Should never happen if plugin was initialized properly.
    UnableToGetMemory = -128,
    /// Window with provided id does not exist.
    Window = -4,
    /// Invalid argument provided.
    BadArgument = -3,
    /// Command execution error.
    Execution = -2,
    /// No information could be provided about this error.
    Unknown = -1,
    /// Success.
    Ok = 0,
}

impl From<X11ReplyError> for ErrorCode {
    fn from(e: X11ReplyError) -> Self {
        match e {
            X11ReplyError::ConnectionError(_) => ErrorCode::Unknown,
            X11ReplyError::X11Error(e) => match e.error_kind {
                x11rb::protocol::ErrorKind::Match => ErrorCode::BadArgument,
                x11rb::protocol::ErrorKind::Window => ErrorCode::Window,
                _ => ErrorCode::Unknown,
            },
        }
    }
}

impl From<X11ConnectionError> for ErrorCode {
    fn from(_: X11ConnectionError) -> Self {
        ErrorCode::Unknown
    }
}

trait ValOrErrCode {
    fn value_or_error_code(self) -> i32;
}

impl ValOrErrCode for Result<u32, ErrorCode> {
    fn value_or_error_code(self) -> i32 {
        match self {
            Ok(u32) => u32 as i32,
            Err(e) => e as i32,
        }
    }
}

impl ValOrErrCode for Result<(), ErrorCode> {
    fn value_or_error_code(self) -> i32 {
        match self {
            Ok(()) => ErrorCode::Ok as i32,
            Err(e) => e as i32,
        }
    }
}
