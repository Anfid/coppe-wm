mod bindings;
mod client;
mod layout;
mod wm;

use crate::bindings::*;
use crate::wm::{Handler, WindowManager};

use x11rb::protocol::xproto::ModMask;

const SYS_MOD: u16 = ModMask::M4 as u16;

fn keys() -> Vec<(Key, Handler)> {
    vec![(
        Key {
            modmask: SYS_MOD,
            keycode: 53u8,
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
    )]
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

    wm.run().unwrap();
}
