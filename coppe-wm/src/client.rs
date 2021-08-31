use x11rb::protocol::xproto::*;

pub struct Client {
    pub window: Window,
    pub frame_window: Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Client {
    pub fn new(window: Window, frame_window: Window, geom: &GetGeometryReply) -> Client {
        Client {
            window,
            frame_window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
        }
    }

    pub fn close_x_position(&self) -> i16 {
        std::cmp::max(0, self.width - 15) as _
    }
}
