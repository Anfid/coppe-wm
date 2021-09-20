use coppe_common::client::Client as CommonClient;
use std::ops::{Deref, DerefMut};
use x11rb::protocol::xproto::*;

#[derive(Debug, Clone, Copy)]
pub struct Client(CommonClient);

impl Client {
    pub fn new(id: Window, geom: &GetGeometryReply) -> Self {
        Self(CommonClient {
            id,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
        })
    }
}

impl Deref for Client {
    type Target = CommonClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
