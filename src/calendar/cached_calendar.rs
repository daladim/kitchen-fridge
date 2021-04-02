use std::collections::{HashMap, HashSet};
use std::error::Error;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::SyncStatus;
use crate::traits::{BaseCalendar, CompleteCalendar};
use crate::calendar::{CalendarId, SupportedComponents};
use crate::Item;
use crate::item::ItemId;


/// A calendar used by the [`cache`](crate::cache) module
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CachedCalendar {
    name: String,
    id: CalendarId,
    supported_components: SupportedComponents,

    items: HashMap<ItemId, Item>,
}

#[async_trait]
impl BaseCalendar for CachedCalendar {
    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &CalendarId {
        &self.id
    }

    fn supported_components(&self) -> SupportedComponents {
        self.supported_components
    }

    async fn add_item(&mut self, item: Item) -> Result<(), Box<dyn Error>> {
        // TODO: here (and in the remote version, display an errror in case we overwrite something?)
        self.items.insert(item.id().clone(), item);
        Ok(())
    }
}

#[async_trait]
impl CompleteCalendar for CachedCalendar {
    fn new(name: String, id: CalendarId, supported_components: SupportedComponents) -> Self {
        Self {
            name, id, supported_components,
            items: HashMap::new(),
        }
    }

    async fn get_item_ids(&self) -> Result<HashSet<ItemId>, Box<dyn Error>> {
        Ok(self.items.iter()
            .map(|(id, _)| id.clone())
            .collect()
        )
    }

    async fn get_items(&self) -> Result<HashMap<ItemId, &Item>, Box<dyn Error>> {
        Ok(self.items.iter()
            .map(|(id, item)| (id.clone(), item))
            .collect()
        )
    }

    async fn get_item_by_id_ref<'a>(&'a self, id: &ItemId) -> Option<&'a Item> {
        self.items.get(id)
    }

    async fn get_item_by_id_mut<'a>(&'a mut self, id: &ItemId) -> Option<&'a mut Item> {
        self.items.get_mut(id)
    }

    async fn mark_for_deletion(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        match self.items.get_mut(item_id) {
            None => Err("no item for this key".into()),
            Some(item) => {
                match item.sync_status() {
                    SyncStatus::Synced(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status( SyncStatus::LocallyDeleted(prev_ss));
                    },
                    SyncStatus::LocallyModified(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status( SyncStatus::LocallyDeleted(prev_ss));
                    },
                    SyncStatus::LocallyDeleted(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status( SyncStatus::LocallyDeleted(prev_ss));
                    },
                    SyncStatus::NotSynced => {
                        // This was never synced to the server, we can safely delete it as soon as now
                        self.items.remove(item_id);
                    },
                };
                Ok(())
            }
        }
    }

    async fn immediately_delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        match self.items.remove(item_id) {
            None => Err(format!("Item {} is absent from this calendar", item_id).into()),
            Some(_) => Ok(())
        }
    }
}



// This class can be used to mock a remote calendar for integration tests

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use crate::{item::VersionTag,
            traits::DavCalendar};

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
#[async_trait]
impl DavCalendar for CachedCalendar {
    async fn get_item_version_tags(&self) -> Result<HashMap<ItemId, VersionTag>, Box<dyn Error>> {
        use crate::item::SyncStatus;

        let mut result = HashMap::new();

        for (id, item) in self.items.iter() {
            let vt = match item.sync_status() {
                SyncStatus::Synced(vt) => vt.clone(),
                _ => {
                    panic!("Mock calendars must contain only SyncStatus::Synced. Got {:?}", item);
                }
            };
            result.insert(id.clone(), vt);
        }

        Ok(result)
    }

    async fn get_item_by_id(&self, id: &ItemId) -> Result<Option<Item>, Box<dyn Error>> {
        Ok(self.items.get(id).cloned())
    }

    async fn delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        self.immediately_delete_item(item_id).await
    }
}
