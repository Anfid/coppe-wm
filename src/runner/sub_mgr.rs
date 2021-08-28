use log::*;
use std::collections::{HashMap, HashSet};

use super::plug_mgr::PluginId;
use crate::events::WmEvent;

#[derive(Debug, Default)]
pub struct SubscriptionManager {
    subs: HashMap<EventSubscription, HashSet<PluginId>>,
}

impl SubscriptionManager {
    pub fn subscribers(&self, ev: &WmEvent) -> impl Iterator<Item = &PluginId> + Clone {
        let sub = EventSubscription::from(ev);

        self.subs
            .get(&sub)
            .map(|subs| subs.iter())
            .into_iter()
            .flatten()
    }

    pub fn subscribe(&mut self, id: PluginId, ev: EventSubscription) {
        if let Some(subs) = self.subs.get_mut(&ev) {
            subs.insert(id);
        } else {
            let mut subs = HashSet::new();
            subs.insert(id);
            self.subs.insert(ev, subs);
        }
    }

    pub fn unsubscribe(&mut self, id: &PluginId, ev: &EventSubscription) {
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

impl From<&WmEvent> for EventSubscription {
    fn from(ev: &WmEvent) -> Self {
        use EventSubscription::*;
        match ev {
            WmEvent::KeyPressed(_) => KeyPressed,
            WmEvent::KeyReleased(_) => KeyReleased,
        }
    }
}
