use x11rb::protocol::xproto::*;

pub struct Client {
    pub window: Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Client {
    pub fn new(window: Window, geom: &GetGeometryReply) -> Client {
        Client {
            window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
        }
    }
}
