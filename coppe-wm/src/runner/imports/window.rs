use log::*;
use wasmer::WasmPtr;
use x11rb::protocol::xproto::*;

use super::{ErrorCode, ValOrErrCode, XEnv};

pub(super) fn window_move(env: &XEnv, window_id: u32, x: i16, y: i16) -> i32 {
    info!("{}: window_move {} to [{}, {}]", env.id, window_id, x, y);
    let aux = ConfigureWindowAux::default().x(x as i32).y(y as i32);

    env.x11
        .conn
        .configure_window(window_id, &aux)
        .map_err(Into::<ErrorCode>::into)
        .and_then(|cookie| cookie.check().map_err(Into::into))
        .value_or_error_code()
}

pub(super) fn window_resize(env: &XEnv, window_id: u32, width: u16, height: u16) -> i32 {
    info!(
        "{}: window_resize {} to [{}, {}]",
        env.id, window_id, width, height
    );
    let aux = ConfigureWindowAux::default()
        .width(width as u32)
        .height(height as u32);

    env.x11
        .conn
        .configure_window(window_id, &aux)
        .map_err(Into::<ErrorCode>::into)
        .and_then(|cookie| cookie.check().map_err(Into::into))
        .value_or_error_code()
}

pub(super) fn window_move_resize(
    env: &XEnv,
    window_id: u32,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
) -> i32 {
    info!(
        "{}: window_move_resize {} to {{x:{},y:{},width:{},height:{}}}",
        env.id, window_id, x, y, width, height
    );
    let aux = ConfigureWindowAux::default()
        .x(x as i32)
        .y(y as i32)
        .width(width as u32)
        .height(height as u32);

    env.x11
        .conn
        .configure_window(window_id, &aux)
        .map_err(Into::<ErrorCode>::into)
        .and_then(|cookie| cookie.check().map_err(Into::into))
        .value_or_error_code()
}

pub(super) fn window_focus(env: &XEnv, window_id: u32) -> i32 {
    info!("{}: window_focus {}", env.id, window_id);
    let aux = ConfigureWindowAux::default().stack_mode(StackMode::ABOVE);

    env.x11
        .conn
        .configure_window(window_id, &aux)
        .map_err(Into::<ErrorCode>::into)
        .and_then(|cookie| cookie.check().map_err(Into::into))
        .value_or_error_code()
}

pub(super) fn window_get_properties(
    env: &XEnv,
    window_id: u32,
    x: WasmPtr<i16>,
    y: WasmPtr<i16>,
    width: WasmPtr<u16>,
    height: WasmPtr<u16>,
) -> i32 {
    info!("{}: window_get_properties {}", env.id, window_id);
    env.memory_ref()
        .ok_or(ErrorCode::UnableToGetMemory)
        .and_then(|memory| {
            let geometry = env.x11.conn.get_geometry(window_id)?.reply()?;
            unsafe {
                x.deref_mut(memory)
                    .ok_or(ErrorCode::BadArgument)?
                    .set(geometry.x);
                y.deref_mut(memory)
                    .ok_or(ErrorCode::BadArgument)?
                    .set(geometry.y);
                width
                    .deref_mut(memory)
                    .ok_or(ErrorCode::BadArgument)?
                    .set(geometry.width);
                height
                    .deref_mut(memory)
                    .ok_or(ErrorCode::BadArgument)?
                    .set(geometry.height);
            };
            Ok(())
        })
        .value_or_error_code()
}

pub(super) fn window_close(env: &XEnv, window_id: u32) -> i32 {
    info!("{}: window_close {}", env.id, window_id);

    let data = [env.x11.atoms.WM_DELETE_WINDOW, 0, 0, 0, 0];
    let event = ClientMessageEvent {
        response_type: CLIENT_MESSAGE_EVENT,
        format: 32,
        sequence: 0,
        window: window_id,
        type_: env.x11.atoms.WM_PROTOCOLS,
        data: data.into(),
    };

    env.x11
        .conn
        .send_event(false, window_id, EventMask::NO_EVENT, &event)
        .map_err(Into::<ErrorCode>::into)
        .and_then(|cookie| cookie.check().map_err(Into::into))
        .value_or_error_code()
}
