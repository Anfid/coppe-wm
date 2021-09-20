use coppe_common::{
    event::{id, Event},
    key::Key,
};
use x11rb::protocol::{xproto::ConfigureWindowAux, Event as XEvent};

pub use coppe_common::event::{Subscription, SubscriptionFilter};

#[derive(Debug, Clone)]
pub struct WmEvent(Event);

impl WmEvent {
    pub fn try_from(x_event: &XEvent) -> Option<Self> {
        match x_event {
            XEvent::KeyPress(event) => Some(
                Event::KeyPress(Key {
                    modmask: event.state.into(),
                    keycode: event.detail.into(),
                })
                .into(),
            ),
            XEvent::KeyRelease(event) => Some(
                Event::KeyRelease(Key {
                    modmask: event.state.into(),
                    keycode: event.detail.into(),
                })
                .into(),
            ),
            _ => None,
        }
    }

    pub fn id(&self) -> u32 {
        use Event::*;
        match self.0 {
            KeyPress(_) => id::KEY_PRESS,
            KeyRelease(_) => id::KEY_RELEASE,
        }
    }

    pub fn matches(&self, _filters: &Vec<SubscriptionFilter>) -> bool {
        // No filters implemented yet
        true
    }
}

impl From<Event> for WmEvent {
    fn from(event: Event) -> Self {
        Self(event)
    }
}

impl From<WmEvent> for Event {
    fn from(ev: WmEvent) -> Self {
        ev.0
    }
}

#[derive(Debug)]
pub enum Command {
    Subscribe(Event),
    Unsubscribe(Event),
    ConfigureWindow(ConfigureWindowAux),
}
