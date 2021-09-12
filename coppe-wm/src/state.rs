use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::sync::Arc;
use wasmer::WasmerEnv;
use x11rb::protocol::xproto::Window;

use crate::client::Client;
use crate::layout::*;

#[derive(Clone, WasmerEnv)]
pub struct State(Arc<RwLock<StateInner>>);

pub struct StateInner {
    pub focused: usize,
    pub layout: Box<dyn Layout>,
    pub clients: Vec<Client>,
}

impl State {
    pub fn get(&self) -> RwLockReadGuard<StateInner> {
        self.0.read()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<StateInner> {
        self.0.write()
    }

    pub fn get_client_by_id(&self, id: Window) -> Option<MappedRwLockReadGuard<Client>> {
        let lock = self.0.read();
        RwLockReadGuard::try_map(lock, |state| state.clients.iter().find(|c| c.window == id)).ok()
    }

    pub fn get_client_by_id_mut(&self, id: Window) -> Option<MappedRwLockWriteGuard<Client>> {
        let lock = self.0.write();
        RwLockWriteGuard::try_map(lock, |state| {
            state.clients.iter_mut().find(|c| c.window == id)
        })
        .ok()
    }
}

impl Default for State {
    fn default() -> Self {
        let inner = Default::default();
        State(Arc::new(RwLock::new(inner)))
    }
}

impl Default for StateInner {
    fn default() -> Self {
        Self {
            focused: 0,
            clients: Vec::new(),
            layout: Box::new(Fullscreen),
        }
    }
}
