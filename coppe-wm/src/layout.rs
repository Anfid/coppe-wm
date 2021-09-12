#![allow(unused)]

use x11rb::protocol::xproto::*;

use std::iter::{FromIterator, IntoIterator};

use crate::client::Client;

pub struct Geometry {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl From<&Screen> for Geometry {
    fn from(screen: &Screen) -> Self {
        Self {
            x: 0,
            y: 0,
            width: screen.width_in_pixels,
            height: screen.height_in_pixels,
        }
    }
}

impl From<&GetGeometryReply> for Geometry {
    fn from(reply: &GetGeometryReply) -> Self {
        Self {
            x: reply.x,
            y: reply.y,
            width: reply.width,
            height: reply.height,
        }
    }
}

pub trait Layout: Send + Sync {
    fn geometry(&self, screen_geo: Geometry, client_geo: Geometry) -> Geometry;
}

pub struct Fullscreen;

impl Layout for Fullscreen {
    fn geometry(&self, screen_geo: Geometry, client_geo: Geometry) -> Geometry {
        screen_geo
    }
}

pub struct Floating;

impl Layout for Floating {
    fn geometry(&self, screen_geo: Geometry, client_geo: Geometry) -> Geometry {
        Geometry {
            x: 0,
            y: 0,
            width: 600,
            height: 400,
        }
    }
}
