use x11rb::protocol::xproto::ButtonIndex;
//use x11rb::protocol::xproto::ModMask;

#[derive(Hash, PartialEq, Eq)]
pub struct Key {
    pub modmask: u16,
    pub keycode: u8,
}

impl From<(u16, u8)> for Key {
    fn from(tup: (u16, u8)) -> Self {
        Self { modmask: tup.0, keycode: tup.1 }
    }
}

// TODO
#[allow(unused)]
pub struct Button {
    pub modmask: u16,
    pub button: ButtonIndex,
}

