pub use coppe_common::window::*;

use crate::ffi;

pub fn move_to(id: WindowId, x: i16, y: i16) {
    ffi::window_move(id, x, y)
}

pub fn resize(id: WindowId, width: u16, height: u16) {
    ffi::window_resize(id, width, height)
}

pub fn focus(id: WindowId) {
    ffi::window_focus(id)
}

pub fn get_geometry(id: WindowId) -> Geometry {
    let mut geometry = Geometry {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    ffi::window_get_properties(
        id,
        &mut geometry.x,
        &mut geometry.y,
        &mut geometry.width,
        &mut geometry.height,
    );

    geometry
}

pub fn set_geometry(id: WindowId, geometry: Geometry) {
    ffi::window_move_resize(id, geometry.x, geometry.y, geometry.width, geometry.height)
}

pub fn close(id: WindowId) {
    ffi::window_close(id)
}
