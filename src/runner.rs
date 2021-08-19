use lazy_static::lazy_static;
use log::*;
use std::{
    fs::File,
    io::Read,
    sync::{mpsc, Mutex},
    thread_local,
};
use wasmer::{
    imports, Array, Function, ImportObject, Instance, LazyInit, Memory, Module, NativeFunc, Store,
    WasmPtr, WasmerEnv,
};

use crate::events::{RunnerEvent, WMEvent};
use crate::state::State;

struct Plugin {
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
        let imports = import_objects(&self.store, self.state.clone());

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
            let mut file = File::open(path).unwrap();

            let mut code = Vec::new();
            file.read_to_end(&mut code).unwrap();

            let module = Module::new(&self.store, &code).unwrap();
            let instance = Instance::new(&module, &imports).unwrap();

            self.plugins.push(Plugin { instance })
        }
    }

    pub fn run(&mut self) {
        self.init_plugins();

        //while let Ok(event) = self.rx.recv() {
        //    //todo!()
        //}
        for plugin in &self.plugins {
            let handle: NativeFunc<(), ()> = plugin
                .instance
                .exports
                .get_native_function("handle")
                .unwrap();
            //let memory = instance.exports.get_memory("memory").unwrap();

            handle.call().unwrap();
        }
    }
}

fn import_objects(store: &Store, state: State) -> ImportObject {
    let environment = Environment {
        wm_state: state,
        memory: Default::default(),
    };

    imports! {
        "env" => {
            "move_window" => Function::new_native(store, move_window),
            "spawn" => Function::new_native_with_env(store, environment, spawn),
        }
    }
}

#[inline]
fn send_event(event: RunnerEvent) {
    thread_local! {
        static S: mpsc::Sender<RunnerEvent> = G.lock().unwrap().0.clone();
    }
    S.with(|sender| sender.send(event));
}

#[derive(WasmerEnv, Clone)]
struct Environment {
    wm_state: State,
    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

fn move_window(id: u32, x: i32, y: i32) {
    send_event(RunnerEvent::MoveWindow { id, x, y });
}

fn spawn(env: &Environment, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) {
    env.memory_ref()
        .and_then(|memory| cmd_ptr.get_utf8_string(memory, cmd_len))
        .and_then(|cmd_string| shlex::split(&cmd_string))
        .filter(|cmd_args| cmd_args.len() > 0)
        .map(|cmd_args| {
            std::process::Command::new(&cmd_args[0])
                .args(&cmd_args[1..])
                .spawn()
        });
}
