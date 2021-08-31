use x11rb::protocol::xproto::ButtonIndex;
use x11rb::protocol::xproto::ModMask;

#[derive(Debug, PartialEq, Eq)]
pub struct Key {
    pub modmask: ModMask,
    pub keycode: u8,
}

impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        u16::from(self.modmask).hash(state);
        self.keycode.hash(state);
    }
}

impl From<(u16, u8)> for Key {
    fn from(tup: (u16, u8)) -> Self {
        Self {
            modmask: ModMask::from(tup.0),
            keycode: tup.1,
        }
    }
}

// TODO
#[allow(unused)]
pub struct Button {
    pub modmask: u16,
    pub button: ButtonIndex,
}
