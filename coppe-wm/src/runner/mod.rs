use log::*;
use std::sync::mpsc;

mod imports;
mod plug_mgr;
mod sub_mgr;

use crate::events::WmEvent;
use crate::x11::X11Info;
use plug_mgr::PluginManager;

pub struct Runner {
    plugins: PluginManager,
    rx: mpsc::Receiver<WmEvent>,
}

impl Runner {
    pub fn new(rx: mpsc::Receiver<WmEvent>, conn: X11Info) -> Self {
        Self {
            plugins: PluginManager::new(conn),
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
