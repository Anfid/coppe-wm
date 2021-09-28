use coppe_std::debug::log;
use coppe_std::event::{self, Event, SubscriptionEvent};
use coppe_std::key::{Key, Keycode, ModMask};
use coppe_std::prelude::*;
use coppe_std::window::WindowId;

static mut WINDOWS: Vec<WindowId> = Vec::new();

#[no_mangle]
pub extern "C" fn init() {
    let mut sub_buffer = [0; 7];

    SubscriptionEvent::key_press(Key::new(ModMask::M4, Keycode::Z))
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::key_release(Key::new(ModMask::M4, Keycode::Z))
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::window_add()
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::window_remove()
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();
}

#[no_mangle]
pub extern "C" fn handle() {
    if let Some(event) = event::read() {
        match event {
            Event::KeyPress(ModMask::M4, Keycode::Z) => {
                log("Win+Z pressed");
            }
            Event::KeyRelease(ModMask::M4, Keycode::Z) => {
                log("Win+Z released");
                list_windows()
            }
            Event::WindowAdd(id) => {
                log(format!("New window: {}", id));
                unsafe { WINDOWS.push(id) }
            }
            Event::WindowRemove(id) => {
                log(format!("Window removed: {}", id));
                unsafe { WINDOWS.retain(|window| window != &id) }
            }
            _ => {}
        }
    }
}

fn list_windows() {
    log(format!("{:?}", unsafe { &WINDOWS }))
}
