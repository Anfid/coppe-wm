use log::*;
use std::{
    fs::File,
    io::Read,
    sync::{Arc, Mutex},
};
use wasmer::{Array, Instance, Module, NativeFunc, Store, Val, WasmPtr};

use crate::events::WMEvent;
use crate::X11Conn;

#[derive(Default)]
pub struct PluginManager {
    store: Store,
    plugins: Vec<Plugin>,
}

pub struct Plugin {
    pub id: String,
    pub instance: Instance,
}

impl Plugin {
    pub fn handle(&self, ev: &WMEvent) {
        let handle: NativeFunc<(), ()> = match self.instance.exports.get_native_function("handle") {
            Ok(func) => func,
            Err(_) => return,
        };
        handle.call().unwrap();
    }
}

impl PluginManager {
    pub fn init(
        &mut self,
        conn: Arc<X11Conn>,
        subscriptions: Arc<Mutex<super::sub_mgr::SubscriptionManager>>,
        state: crate::state::State,
    ) {
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

        let imports =
            super::imports::import_objects(&self.store, conn, subscriptions.clone(), state.clone());

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
                id
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

            self.plugins.push(Plugin { id, instance })
        }
    }

    pub fn get(&self, id: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|plugin| plugin.id == id)
    }
}
