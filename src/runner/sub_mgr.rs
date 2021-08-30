use log::*;
use std::collections::HashMap;

use super::plug_mgr::PluginId;
use crate::events::{Subscription, SubscriptionEvent, SubscriptionFilterGroup, WmEvent};

#[derive(Debug, Default)]
pub struct SubscriptionManager {
    subs: HashMap<SubscriptionEvent, HashMap<PluginId, Vec<SubscriptionFilterGroup>>>,
}

impl SubscriptionManager {
    pub fn subscribers(&self, ev: &WmEvent) -> Vec<&PluginId> {
        let sub = SubscriptionEvent::from(ev);

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
        match self.subs.entry(sub.event) {
            Entry::Occupied(mut event_subs) => match event_subs.get_mut().entry(id) {
                Entry::Occupied(mut filters) => filters.get_mut().push(sub.filters),
                Entry::Vacant(filters) => {
                    // TODO: Subscribe to X event
                    filters.insert(vec![sub.filters]);
                }
            },
            Entry::Vacant(event_subs) => {
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
                // TODO: Unsubscribe from X event
                self.subs.remove(&unsub.event);
            }
        }
    }
}
