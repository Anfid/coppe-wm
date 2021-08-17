mod bindings;
mod client;
mod layout;
mod runner;
mod wm;

use crate::bindings::*;
use crate::runner::Runner;
use crate::wm::{Handler, WindowManager};

use x11rb::protocol::xproto::ModMask;

const SYS_MOD: ModMask = ModMask::M4;

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

fn main() {
    env_logger::init();
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let mut wm = WindowManager::init(&conn, screen_num).unwrap();
    wm.bind_keys(keys()).unwrap();

    std::process::Command::new("feh")
        .arg("--bg-scale")
        .arg("/home/anfid/Pictures/Wallpapers/Sth2.png")
        .spawn()
        .unwrap();

    Runner::init().unwrap().run();

    wm.run().unwrap();
}
