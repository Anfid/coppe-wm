use coppe_std::debug::log;
use coppe_std::event::{self, Event, SubscriptionEvent};
use coppe_std::key::{Key, Keycode, ModMask};

#[no_mangle]
pub static id: [u8; 6] = *b"debug\0";

#[no_mangle]
pub extern "C" fn init() {
    let mut sub_buffer = [0; 7];

    SubscriptionEvent::key_press(Key::new(ModMask::M4, Keycode::Z))
        .init_without_filters(&mut sub_buffer)
        .unwrap()
        .subscribe();
}

#[no_mangle]
pub extern "C" fn handle() {
    if let Some(event) = event::read() {
        match event {
            Event::KeyPress(ModMask::M4, Keycode::Z) => main(),
            _ => {}
        }
    }
}

fn main() {
    match coppe_std::state::clients::read() {
        Ok(clients) => {
            for client in clients {
                log(format!("Got client: {:?}", client));
            }
        }
        Err(_) => log("Error reading clients"),
    }
}
