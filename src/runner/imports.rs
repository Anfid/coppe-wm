use log::*;
use wasmer::{imports, Array, Function, ImportObject, Store, WasmPtr};

use super::{send_event, Environment};
use crate::events::RunnerEvent;
use crate::state::State;

pub(super) fn import_objects(store: &Store, state: State) -> ImportObject {
    let environment = Environment {
        wm_state: state,
        memory: Default::default(),
        id: Default::default(),
    };

    imports! {
        "env" => {
            "move_window" => Function::new_native_with_env(store, environment.clone(), move_window),
            "spawn" => Function::new_native_with_env(store, environment.clone(), spawn),
        }
    }
}

fn move_window(env: &Environment, id: u32, x: i32, y: i32) {
    let plugin_id = env.read_id();
    info!("move_window {} to [{}, {}] from {:?}", id, x, y, plugin_id);

    send_event(RunnerEvent::MoveWindow { id, x, y });
}

fn spawn(env: &Environment, cmd_ptr: WasmPtr<u8, Array>, cmd_len: u32) {
    env.memory_ref()
        .and_then(|memory| {
            let id = env.read_id()?;
            let cmd_string = cmd_ptr.get_utf8_string(memory, cmd_len)?;
            info!("spawn '{}' from {}", cmd_string, id);
            Some(cmd_string)
        })
        .and_then(|cmd_string| shlex::split(&cmd_string))
        .filter(|cmd_args| cmd_args.len() > 0)
        .map(|cmd_args| {
            std::process::Command::new(&cmd_args[0])
                .args(&cmd_args[1..])
                .spawn()
        });
}
