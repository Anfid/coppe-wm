use coppe_common::client::Client as CommonClient;
use std::ops::{Deref, DerefMut};
use x11rb::protocol::xproto::*;

pub use coppe_common::client::{ClientGeometry, ClientId};

#[derive(Debug, Clone, Copy)]
pub struct Client(CommonClient);

impl Client {
    pub fn new(id: Window, geom: &GetGeometryReply) -> Self {
        let geometry = ClientGeometry {
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
        };
        Self(CommonClient { id, geometry })
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
