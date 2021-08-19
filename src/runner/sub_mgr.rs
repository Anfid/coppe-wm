use log::*;
use std::collections::{HashMap, HashSet};

use super::plug_mgr::PluginManager;
use crate::events::WMEvent;

#[derive(Debug, Default)]
pub struct SubscriptionManager {
    subs: HashMap<EventSubscription, HashSet<String>>,
}

impl SubscriptionManager {
    pub fn dispatch(&self, ev: WMEvent, plug_mgr: &PluginManager) {
        let sub = EventSubscription::from(&ev);
        if let Some(subscribers) = self.subs.get(&sub) {
            for subscriber in subscribers {
                info!("Handling event {:?} by {}", ev, subscriber);
                plug_mgr.get(subscriber).map(|s| s.handle(&ev));
            }
        }
    }

    pub fn subscribe(&mut self, id: String, ev: EventSubscription) {
        if let Some(subs) = self.subs.get_mut(&ev) {
            subs.insert(id);
        } else {
            let mut subs = HashSet::new();
            subs.insert(id);
            self.subs.insert(ev, subs);
        }
    }

    pub fn unsubscribe(&mut self, id: &String, ev: &EventSubscription) {
        if let Some(subs) = self.subs.get_mut(ev) {
            subs.remove(id);
            if subs.is_empty() {
                self.subs.remove(ev);
            }
        }
    }
}

// TODO: event filters
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum EventSubscription {
    KeyPressed = 1,
    KeyReleased = 2,
    ManageClient = 3,
    UnmanageClient = 4,
}

impl EventSubscription {
    pub fn try_from(id: u32) -> Option<Self> {
        match id {
            1 => Some(Self::KeyPressed),
            2 => Some(Self::KeyReleased),
            3 => Some(Self::ManageClient),
            4 => Some(Self::UnmanageClient),
            _ => None,
        }
    }
}

impl From<&WMEvent> for EventSubscription {
    fn from(ev: &WMEvent) -> Self {
        use EventSubscription::*;
        match ev {
            WMEvent::KeyPressed(_) => KeyPressed,
            WMEvent::KeyReleased(_) => KeyReleased,
        }
    }
}
