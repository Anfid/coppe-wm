#![no_std]

use coppe_core::command;
use coppe_core::event::{self, Event};
use coppe_core::key::{Key, Keycode, ModMask};

#[no_mangle]
pub static id: [u8; 10] = *b"rust_demo\0";

#[no_mangle]
pub extern "C" fn init() {
    let mut sub_buffer = [0; 3];
    Key::new(ModMask::M4, Keycode::Return)
        .press_subscription()
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    Key::new(ModMask::M4, Keycode::X)
        .press_subscription()
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    command::spawn("feh --bg-scale /home/anfid/Pictures/Wallpapers/Sth2.png");
}

#[no_mangle]
pub extern "C" fn handle() {
    let terminal = "kitty";
    let rofi = "rofi -modi drun,run -show run -location 0 -xoffset 0";

    if let Some(event) = event::read_parse() {
        match event {
            Event::KeyPress(ModMask::M4, Keycode::Return) => command::spawn(terminal),
            Event::KeyPress(ModMask::M4, Keycode::X) => command::spawn(rofi),
            _ => {}
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { core::arch::wasm32::unreachable() }
}
