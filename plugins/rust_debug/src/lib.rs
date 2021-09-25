use coppe_std::client::ClientId;
use coppe_std::debug::log;
use coppe_std::event::{self, Event, SubscriptionEvent};
use coppe_std::key::{Key, Keycode, ModMask};
use coppe_std::prelude::*;

static mut CLIENTS: Vec<ClientId> = Vec::new();

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

    SubscriptionEvent::client_add()
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();

    SubscriptionEvent::client_remove()
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
                list_clients()
            }
            Event::ClientAdd(id) => {
                log(format!("New client: {}", id));
                unsafe { CLIENTS.push(id) }
            }
            Event::ClientRemove(id) => {
                log(format!("Client removed: {}", id));
                unsafe { CLIENTS.retain(|client| client != &id) }
            }
            _ => {}
        }
    }
}

fn list_clients() {
    log(format!("{:?}", unsafe { &CLIENTS }))
}
