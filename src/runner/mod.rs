use lazy_static::lazy_static;
use log::*;
use std::{
    fs::File,
    io::Read,
    sync::{mpsc, Mutex},
    thread_local,
};
use wasmer::{
    Array, Global, Instance, LazyInit, Memory, Module, NativeFunc, Store, Val, WasmPtr, WasmerEnv,
};

mod imports;

use crate::events::{RunnerEvent, WMEvent};
use crate::state::State;

struct Plugin {
    id: String,
    instance: Instance,
}

pub struct Runner {
    store: Store,
    plugins: Vec<Plugin>,
    state: State,
    rx: mpsc::Receiver<WMEvent>,
}

lazy_static! {
    static ref G: Mutex<(
        mpsc::Sender<RunnerEvent>,
        Option<mpsc::Receiver<RunnerEvent>>
    )> = {
        let (tx, rx) = mpsc::channel();
        Mutex::new((tx, Some(rx)))
    };
}

impl Runner {
    pub fn init(state: State, rx: mpsc::Receiver<WMEvent>) -> (Self, mpsc::Receiver<RunnerEvent>) {
        let runner = Runner {
            store: Store::default(),
            plugins: Vec::new(),
            state,
            rx,
        };

        let rx = G.lock().unwrap().1.take().unwrap();
        (runner, rx)
    }

    pub fn init_plugins(&mut self) {
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

        for plugin_dir_entry in std::fs::read_dir(&plugin_dir).unwrap() {
            let path = plugin_dir_entry.unwrap().path();
            let mut file = File::open(&path).unwrap();

            let mut code = Vec::new();
            file.read_to_end(&mut code).unwrap();

            let imports = imports::import_objects(&self.store, self.state.clone());
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
            }

            self.plugins.push(Plugin { id, instance })
        }
    }

    pub fn run(&mut self) {
        self.init_plugins();

        for plugin in &self.plugins {
            let handle: NativeFunc<(), ()> =
                match plugin.instance.exports.get_native_function("handle") {
                    Ok(func) => func,
                    Err(_) => continue,
                };
            handle.call().unwrap();
        }

        while let Ok(event) = self.rx.recv() {}
    }
}

#[inline]
fn send_event(event: RunnerEvent) {
    thread_local! {
        static S: mpsc::Sender<RunnerEvent> = G.lock().unwrap().0.clone();
    }
    let res = S.with(|sender| sender.send(event));
    if let Err(e) = res {
        warn!("Unable to send event to WM: {}", e)
    }
}

#[derive(WasmerEnv, Clone)]
struct Environment {
    wm_state: State,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    #[wasmer(export)]
    id: LazyInit<Global>,
}

impl Environment {
    fn read_id(&self) -> Option<String> {
        self.id_ref()
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
                ptr.get_utf8_string_with_nul(&self.memory_ref().unwrap())
            })
    }
}
