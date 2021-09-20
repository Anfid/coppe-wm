use coppe_common::event::{Event, Subscription, SubscriptionFilter};
use log::*;
use std::{collections::HashMap, sync::mpsc::SyncSender};

use super::plug_mgr::PluginId;
use crate::events::{Command, WmEvent};

#[derive(Debug)]
pub struct SubscriptionManager {
    subs: HashMap<Event, HashMap<PluginId, Vec<Vec<SubscriptionFilter>>>>,
    conn: SyncSender<Command>,
}

impl SubscriptionManager {
    pub fn new(command_tx: SyncSender<Command>) -> Self {
        Self {
            conn: command_tx,
            subs: Default::default(),
        }
    }
}

impl SubscriptionManager {
    pub fn subscribers(&self, ev: &WmEvent) -> Vec<&PluginId> {
        let sub = Event::from(ev.clone());

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
                self.conn.send(Command::Subscribe(sub.event)).unwrap();
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
                self.conn
                    .send(Command::Unsubscribe(unsub.event.clone()))
                    .unwrap();
                self.subs.remove(&unsub.event);
            }
        }
    }
}
