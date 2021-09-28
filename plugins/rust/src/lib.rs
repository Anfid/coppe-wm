use coppe_std::command;
use coppe_std::event::{self, Event, SubscriptionEvent};
use coppe_std::key::{Key, Keycode, ModMask};
use coppe_std::prelude::*;

#[no_mangle]
pub extern "C" fn init() {
    let mut sub_buffer = [0; 7];

    SubscriptionEvent::key_press(Key::new(ModMask::M4, Keycode::Return))
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::key_press(Key::new(ModMask::M4, Keycode::X))
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    command::spawn("feh --bg-scale /home/anfid/Pictures/Wallpapers/Sth2.png");
}

#[no_mangle]
pub extern "C" fn handle() {
    let terminal = "kitty";
    let rofi = "rofi -modi drun,run -show run -location 0 -xoffset 0";

    if let Some(event) = event::read() {
        match event {
            Event::KeyPress(ModMask::M4, Keycode::Return) => command::spawn(terminal),
            Event::KeyPress(ModMask::M4, Keycode::X) => command::spawn(rofi),
            _ => {}
        }
    }
}
