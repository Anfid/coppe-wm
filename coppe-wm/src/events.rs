use coppe_common::{
    event::{Event, SubscriptionEvent},
    key::Key,
    window::{Geometry, Window},
};
use x11rb::protocol::Event as XEvent;

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
            XEvent::MapRequest(event) => Some(Event::WindowAdd(event.window).into()),
            XEvent::UnmapNotify(event) => Some(Event::WindowRemove(event.window).into()),
            XEvent::ConfigureNotify(event) => Some(
                Event::WindowConfigure(Window {
                    id: event.window,
                    geometry: Geometry {
                        x: event.x,
                        y: event.y,
                        width: event.width,
                        height: event.height,
                    },
                })
                .into(),
            ),
            _ => None,
        }
    }

    pub fn id(&self) -> u32 {
        self.0.id()
    }

    pub fn matches(&self, _filters: &[SubscriptionFilter]) -> bool {
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

impl From<&WmEvent> for SubscriptionEvent {
    fn from(event: &WmEvent) -> Self {
        SubscriptionEvent::from(&event.0)
    }
}
