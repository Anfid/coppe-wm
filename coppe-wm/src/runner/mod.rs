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
    pub fn init(conn: X11Info, rx: mpsc::Receiver<WmEvent>) -> Self {
        Self {
            plugins: PluginManager::init(conn),
            rx,
        }
    }

    pub fn run(&mut self) {
        while let Ok(event) = self.rx.recv() {
            info!("Dispatching event {:?}", event);
            self.plugins.handle(event)
        }
    }
}
