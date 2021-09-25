use x11rb::protocol::xproto::Window;

use crate::client::Client;
use crate::layout::*;

pub struct State {
    pub focused: usize,
    pub layout: Box<dyn Layout>,
    pub clients: Vec<Client>,
}

impl State {
    pub fn get_client_by_id(&self, id: Window) -> Option<&Client> {
        self.clients.iter().find(|c| c.id == id)
    }

    pub fn get_client_by_id_mut(&mut self, id: Window) -> Option<&mut Client> {
        self.clients.iter_mut().find(|c| c.id == id)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            focused: 0,
            clients: Vec::new(),
            layout: Box::new(Fullscreen),
        }
    }
}
