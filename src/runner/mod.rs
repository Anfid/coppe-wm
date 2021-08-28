use log::*;
use std::sync::{mpsc, Arc};

mod imports;
mod plug_mgr;
mod sub_mgr;

use crate::events::WmEvent;
use crate::state::State;
use crate::X11Conn;
use plug_mgr::PluginManager;

pub struct Runner {
    plugins: PluginManager,
    state: State,
    rx: mpsc::Receiver<WmEvent>,
}

impl Runner {
    pub fn new(state: State, rx: mpsc::Receiver<WmEvent>) -> Self {
        Self {
            plugins: Default::default(),
            state,
            rx,
        }
    }

    pub fn run(&mut self, conn: Arc<X11Conn>) {
        self.plugins.init(conn, self.state.clone());

        while let Ok(event) = self.rx.recv() {
            info!("Dispatching event {:?}", event);
            self.plugins.handle(event)
        }
    }
}
