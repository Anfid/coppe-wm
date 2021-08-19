use log::*;
use std::sync::{mpsc, Arc, Mutex};

mod imports;
mod plug_mgr;
mod sub_mgr;

use crate::events::WMEvent;
use crate::state::State;
use crate::X11Conn;
use plug_mgr::PluginManager;
use sub_mgr::SubscriptionManager;

pub struct Runner {
    plugins: PluginManager,
    subscriptions: Arc<Mutex<SubscriptionManager>>,
    state: State,
    rx: mpsc::Receiver<WMEvent>,
}

impl Runner {
    pub fn new(state: State, rx: mpsc::Receiver<WMEvent>) -> Self {
        Self {
            plugins: Default::default(),
            subscriptions: Default::default(),
            state,
            rx,
        }
    }

    pub fn run(&mut self, conn: Arc<X11Conn>) {
        self.plugins
            .init(conn, self.subscriptions.clone(), self.state.clone());

        while let Ok(event) = self.rx.recv() {
            info!("Dispatching event {:?}", event);
            self.subscriptions
                .lock()
                .unwrap()
                .dispatch(event, &self.plugins)
        }
    }
}
