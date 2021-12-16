use coppe_common::event::Event;
use log::*;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Display},
    fs::File,
    io::Read,
    path::PathBuf,
    sync::Arc,
};
use wasmer::{Instance, Module, NativeFunc, Store};

use super::imports;
use super::sub_mgr::SubscriptionManager;
use crate::events::WmEvent;
use crate::x11::X11Info;

pub struct PluginManager {
    store: Store,
    instances: HashMap<PluginId, Instance>,
    events: Arc<RwLock<HashMap<PluginId, Mutex<VecDeque<Event>>>>>,
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    x11: X11Info,
}

impl PluginManager {
    pub fn init(x11: X11Info) -> Self {
        let mut plugin_manager = Self {
            store: Default::default(),
            instances: Default::default(),
            events: Default::default(),
            subscriptions: Arc::new(RwLock::new(SubscriptionManager::new(x11.clone()))),
            x11,
        };

        let user_config_dir = PluginManager::get_user_config_dir();
        let plugin_dirs = match std::fs::read_dir(&user_config_dir) {
            Ok(iter) => Some(iter),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        warn!("User configuration directory does not exist")
                    }
                    _ => error!("Unable to access user configuration directory: {}", e),
                }

                let global_config_dir = PluginManager::get_global_config_dir();
                match std::fs::read_dir(&global_config_dir) {
                    Ok(iter) => Some(iter),
                    Err(e) => {
                        match e.kind() {
                            std::io::ErrorKind::NotFound => {
                                error!("Global configuration directory does not exist")
                            }
                            _ => error!("Unable to access global configuration directory: {}", e),
                        }
                        info!("Starting without plugins");
                        None
                    }
                }
            }
        }
        .into_iter()
        .flatten();

        for plugin_dir_entry in plugin_dirs {
            let path = plugin_dir_entry.unwrap().path();

            let id: PluginId =
                if let Some(plugin_name) = path.file_stem().and_then(|stem| stem.to_str()) {
                    plugin_name.into()
                } else {
                    continue;
                };

            let imports = imports::import_objects(
                id.clone(),
                &plugin_manager.store,
                plugin_manager.x11.clone(),
                plugin_manager.subscriptions.clone(),
                plugin_manager.events.clone(),
            );

            info!("Trying to initialize {}", path.to_string_lossy());

            let mut file = File::open(&path).unwrap();

            let mut code = Vec::new();
            file.read_to_end(&mut code).unwrap();

            let module = match Module::new(&plugin_manager.store, &code) {
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

            if let Ok(init) = instance.exports.get_native_function::<(), ()>("init") {
                init.call().unwrap();
                info!("Initialized {}", id);
            }

            plugin_manager.instances.insert(id, instance);
        }

        plugin_manager
    }

    fn get_user_config_dir() -> PathBuf {
        std::env::var("XDG_CONFIG_HOME")
            .map(|path| {
                let mut path = PathBuf::from(path);
                path.push("coppe-wm");
                path
            })
            .or_else(|_| {
                std::env::var("HOME").map(|path| {
                    let mut path = PathBuf::from(path);
                    path.push(".config");
                    path.push("coppe-wm");
                    path
                })
            })
            .unwrap()
    }

    /// Return global configration directory, preserving the installation prefix.
    ///
    /// TODO: consider cases when binary is launched without installation.
    fn get_global_config_dir() -> PathBuf {
        let mut path =
            std::env::current_exe().expect("TODO something something unable to get bin path");

        path.pop();
        path.pop();
        path.push("share");
        path.push("coppe-wm");

        path
    }

    pub fn handle(&self, ev: WmEvent) {
        let sub_lock = self.subscriptions.read();
        let subs = sub_lock.subscribers(&ev);

        for subscriber in &subs {
            // TODO: optimize locks and clones for read acces
            self.events
                .write()
                .entry((*subscriber).clone())
                .or_default()
                .lock()
                .push_back(ev.clone().into());
        }

        for subscriber in subs {
            info!("Calling handle on {}; Reason: event {:?}", subscriber, ev);

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginId(String);

impl From<String> for PluginId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for PluginId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
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
