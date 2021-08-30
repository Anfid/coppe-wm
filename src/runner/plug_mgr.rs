use log::*;
use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Display},
    fs::File,
    io::Read,
    sync::{mpsc, Arc, Mutex, RwLock},
};
use wasmer::{Array, Instance, Module, NativeFunc, Store, Val, WasmPtr};

use super::imports;
use super::sub_mgr::SubscriptionManager;
use crate::events::{Command, EncodedEvent, WmEvent};
use crate::state::State;

pub struct PluginManager {
    store: Store,
    instances: HashMap<PluginId, Instance>,
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<EncodedEvent>>>>>,
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    command_tx: mpsc::SyncSender<Command>,
}

impl PluginManager {
    pub fn new(command_tx: mpsc::SyncSender<Command>) -> Self {
        Self {
            store: Default::default(),
            instances: Default::default(),
            events: Default::default(),
            subscriptions: Arc::new(RwLock::new(SubscriptionManager::new(command_tx.clone()))),
            command_tx,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginId(String);

impl From<String> for PluginId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl PartialEq<&str> for PluginId {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == other
    }
}

impl Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PluginManager {
    pub fn init(&mut self, state: State) {
        let plugin_dir = std::env::var("XDG_CONFIG_HOME")
            .map(|path| {
                let mut path = std::path::PathBuf::from(path);
                path.push("waswm");
                path
            })
            .or_else(|_| {
                std::env::var("HOME").map(|path| {
                    let mut path = std::path::PathBuf::from(path);
                    path.push(".config");
                    path.push("waswm");
                    path
                })
            })
            .unwrap();

        let imports = imports::import_objects(
            &self.store,
            self.command_tx.clone(),
            self.subscriptions.clone(),
            self.events.clone(),
            state.clone(),
        );

        for plugin_dir_entry in std::fs::read_dir(&plugin_dir).unwrap() {
            let path = plugin_dir_entry.unwrap().path();
            info!("Trying to initialize {}", path.to_string_lossy());

            let mut file = File::open(&path).unwrap();

            let mut code = Vec::new();
            file.read_to_end(&mut code).unwrap();

            let module = match Module::new(&self.store, &code) {
                Ok(module) => module,
                Err(e) => {
                    warn!("Plugin '{}' is invalid WASM: {}", path.to_string_lossy(), e);
                    continue;
                }
            };
            let instance = match Instance::new(&module, &imports) {
                Ok(instance) => instance,
                Err(e) => {
                    warn!("Plugin '{}' is incompatible: {}", path.to_string_lossy(), e);
                    continue;
                }
            };

            let id: Option<String> = instance
                .exports
                .get_global("id")
                .ok()
                .map(|g| g.get())
                .and_then(|val| {
                    if let Val::I32(val) = val {
                        Some(val as u32)
                    } else {
                        None
                    }
                })
                .map(|offset| WasmPtr::new(offset))
                .and_then(|ptr: WasmPtr<u8, Array>| {
                    ptr.get_utf8_string_with_nul(&instance.exports.get_memory("memory").unwrap())
                });

            let id = if let Some(id) = id {
                id.into()
            } else {
                warn!(
                    "Plugin '{}' global 'id' could not be parsed",
                    path.to_string_lossy()
                );
                continue;
            };

            if let Ok(init) = instance.exports.get_native_function::<(), ()>("init") {
                init.call().unwrap();
                info!("Initialized {}", id);
            }

            self.instances.insert(id, instance);
        }
    }

    pub fn handle(&self, ev: WmEvent) {
        let sub_lock = self.subscriptions.read().unwrap();
        let subs = sub_lock.subscribers(&ev);
        for subscriber in &subs {
            // TODO: optimize locks and clones for read acces
            self.events
                .write()
                .unwrap()
                .entry((*subscriber).clone())
                .or_default()
                .lock()
                .unwrap()
                .push_back((&ev).into());
        }

        for subscriber in subs {
            info!("Handling event {:?} by {}", ev, subscriber);

            if let Some(instance) = self.instances.get(subscriber) {
                let handle: NativeFunc<(), ()> =
                    match instance.exports.get_native_function("handle") {
                        Ok(func) => func,
                        Err(e) => {
                            warn!(
                                "Unable to get function `handle` for plugin {}: {}",
                                subscriber, e
                            );
                            continue;
                        }
                    };
                handle.call().unwrap();
            } else {
                error!("Unable to find instance for subscriber {}", subscriber);
            }
        }
    }
}
