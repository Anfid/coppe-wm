use log::*;
use std::sync::mpsc;

mod imports;
mod plug_mgr;
mod sub_mgr;

use crate::events::{Command, WmEvent};
use plug_mgr::PluginManager;

pub struct Runner {
    plugins: PluginManager,
    rx: mpsc::Receiver<WmEvent>,
}

impl Runner {
    pub fn new(rx: mpsc::Receiver<WmEvent>, command_tx: mpsc::SyncSender<Command>) -> Self {
        Self {
            plugins: PluginManager::new(command_tx),
            rx,
        }
    }

    pub fn run(&mut self) {
        self.plugins.init();

        while let Ok(event) = self.rx.recv() {
            info!("Dispatching event {:?}", event);
            self.plugins.handle(event)
        }
    }
}
