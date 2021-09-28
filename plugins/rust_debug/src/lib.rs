use coppe_std::debug::log;
use coppe_std::event::{self, Event, SubscriptionEvent};
use coppe_std::key::{Key, Keycode, ModMask};
use coppe_std::prelude::*;
use coppe_std::window::{Geometry, WindowId};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref WINDOWS: Mutex<HashMap<WindowId, Option<Geometry>>> = Mutex::new(HashMap::new());
}

#[no_mangle]
pub extern "C" fn init() {
    let mut sub_buffer = [0; 7];

    SubscriptionEvent::KeyPress(Key::new(ModMask::M4, Keycode::Z))
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::KeyRelease(Key::new(ModMask::M4, Keycode::Z))
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::WindowAdd
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::WindowRemove
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::WindowConfigure
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
                WINDOWS.lock().unwrap().insert(id, None);
            }
            Event::WindowRemove(id) => {
                log(format!("Window removed: {}", id));
                WINDOWS.lock().unwrap().remove(&id);
            }
            Event::WindowConfigure(id, geometry) => {
                log(format!("Window updated: {}, {:?}", id, geometry));
                WINDOWS.lock().unwrap().insert(id, Some(geometry));
            }
            _ => {}
        }
    }
}

fn list_windows() {
    log(format!("{:?}", WINDOWS.lock().unwrap()))
}
