use coppe_common::event::{Subscription, SubscriptionEvent, SubscriptionFilter};
use log::*;
use std::collections::HashMap;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

use super::plug_mgr::PluginId;
use crate::events::WmEvent;
use crate::x11::X11Info;

#[derive(Debug)]
pub struct SubscriptionManager {
    subs: HashMap<SubscriptionEvent, HashMap<PluginId, Vec<Vec<SubscriptionFilter>>>>,
    x11: X11Info,
}

impl SubscriptionManager {
    pub fn new(x11: X11Info) -> Self {
        Self {
            x11,
            subs: Default::default(),
        }
    }
}

impl SubscriptionManager {
    pub fn subscribers(&self, ev: &WmEvent) -> Vec<&PluginId> {
        let sub: SubscriptionEvent = ev.into();

        self.subs
            .get(&sub)
            .map(|subs| {
                subs.iter()
                    .filter(|(_, filters)| {
                        filters.iter().any(|filter_group| ev.matches(filter_group))
                    })
                    .map(|(id, _)| id)
            })
            .into_iter()
            .flatten()
            .collect::<Vec<&PluginId>>()
    }

    pub fn subscribe(&mut self, id: PluginId, sub: Subscription) {
        use std::collections::hash_map::Entry;
        match self.subs.entry(sub.event.clone()) {
            Entry::Occupied(mut event_subs) => match event_subs.get_mut().entry(id) {
                Entry::Occupied(mut filters) => filters.get_mut().push(sub.filters),
                Entry::Vacant(filters) => {
                    filters.insert(vec![sub.filters]);
                }
            },
            Entry::Vacant(event_subs) => {
                info!("Initializing X subscription for {:?}", sub.event);
                use SubscriptionEvent::*;
                match sub.event {
                    KeyPress(key) | KeyRelease(key) => {
                        self.x11
                            .conn
                            .grab_key(
                                true,
                                // FIXME: screen_num should not be hardcoded
                                self.x11.conn.setup().roots[0].root,
                                key.modmask,
                                key.keycode,
                                GrabMode::ASYNC,
                                GrabMode::ASYNC,
                            )
                            .unwrap();
                    }
                    WindowAdd | WindowRemove => {}
                }
                let mut sub_desc = HashMap::new();
                sub_desc.insert(id, vec![sub.filters]);
                event_subs.insert(sub_desc);
            }
        }
    }

    pub fn unsubscribe(&mut self, id: &PluginId, unsub: &Subscription) {
        if let Some(subs) = self.subs.get_mut(&unsub.event) {
            if let Some(filters) = subs.get_mut(id) {
                if unsub.filters.is_empty() {
                    subs.remove(id);
                } else {
                    filters.retain(|group| group != &unsub.filters);

                    if filters.is_empty() {
                        subs.remove(id);
                    }
                }
            }

            if subs.is_empty() {
                info!("Uninitializing X subscription for {:?}", unsub.event);
                use SubscriptionEvent::*;
                match unsub.event {
                    KeyPress(key) | KeyRelease(key) => {
                        self.x11
                            .conn
                            .ungrab_key(
                                key.keycode,
                                self.x11.conn.setup().roots[self.x11.screen_num].root,
                                key.modmask,
                            )
                            .unwrap();
                    }
                    WindowAdd | WindowRemove => {}
                }
                self.subs.remove(&unsub.event);
            }
        }
    }
}
