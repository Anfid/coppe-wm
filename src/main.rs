mod bindings;
mod client;
mod events;
mod layout;
mod runner;
mod state;
mod wm;

use crate::bindings::*;
use crate::runner::Runner;
use crate::state::State;
use crate::wm::{Handler, WindowManager};

use std::sync::Arc;
use x11rb::protocol::xproto::ModMask;

const SYS_MOD: ModMask = ModMask::M4;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let state = State::default();
    let (wm_tx, wm_rx) = std::sync::mpsc::channel();
    let (mut runner, runner_rx) = Runner::init(state.clone(), wm_rx);

    std::thread::spawn(move || runner.run());

    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let conn = Arc::new(conn);
    let mut wm = WindowManager::init(conn.clone(), screen_num, state, wm_tx, runner_rx).unwrap();
    wm.bind_keys(&*conn, keys()).unwrap();

    std::process::Command::new("feh")
        .arg("--bg-scale")
        .arg("/home/anfid/Pictures/Wallpapers/Sth2.png")
        .spawn()
        .unwrap();

    wm.run(&*conn).unwrap();
}

fn keys() -> Vec<(Key, Handler)> {
    vec![
        (
            Key {
                modmask: SYS_MOD,
                keycode: 53,
            },
            Box::new(|| {
                std::process::Command::new("rofi")
                    .args(&[
                        "-modi",
                        "drun,run",
                        "-show",
                        "run",
                        "-location",
                        "0",
                        "-xoffset",
                        "0",
                    ])
                    .spawn()
                    .unwrap();
            }),
        ),
        (
            Key {
                modmask: SYS_MOD,
                keycode: 36,
            },
            Box::new(|| {
                std::process::Command::new("kitty").spawn().unwrap();
            }),
        ),
    ]
}
