use std::collections::{HashMap, HashSet};
use std::collections::BTreeMap;
use std::error::Error;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use async_trait::async_trait;

use crate::traits::{PartialCalendar, CompleteCalendar};
use crate::calendar::{CalendarId, SupportedComponents, SearchFilter};
use crate::Item;
use crate::item::ItemId;
use crate::item::VersionTag;


/// A calendar used by the [`cache`](crate::cache) module
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CachedCalendar {
    name: String,
    id: CalendarId,
    supported_components: SupportedComponents,

    items: HashMap<ItemId, Item>,
    deleted_items: BTreeMap<DateTime<Utc>, ItemId>,
}

impl CachedCalendar {
    /// Create a new calendar
    pub fn new(name: String, id: CalendarId, supported_components: SupportedComponents) -> Self {
        Self {
            name, id, supported_components,
            items: HashMap::new(),
            deleted_items: BTreeMap::new(),
        }
    }
}

#[async_trait]
impl PartialCalendar for CachedCalendar {
    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &CalendarId {
        &self.id
    }

    fn supported_components(&self) -> SupportedComponents {
        self.supported_components
    }

    async fn add_item(&mut self, item: Item) {
        self.items.insert(item.id().clone(), item);
    }

    async fn delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        if let None = self.items.remove(item_id) {
            return Err("This key does not exist.".into());
        }
        self.deleted_items.insert(Utc::now(), item_id.clone());
        Ok(())
    }

    #[cfg(not(feature = "mock_version_tag"))]
    #[allow(unreachable_code)]
    async fn get_item_version_tags(&self) -> Result<HashMap<ItemId, VersionTag>, Box<dyn Error>> {
        panic!("This function only makes sense in remote calendars and in mocked calendars");
        Err("This function only makes sense in remote calendars and in mocked calendars".into())
    }
    #[cfg(feature = "mock_version_tag")]
    async fn get_item_version_tags(&self) -> Result<HashMap<ItemId, VersionTag>, Box<dyn Error>> {
        use crate::item::SyncStatus;

        let mut result = HashMap::new();

        for (id, item) in &self.items {
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

    async fn get_item_by_id_mut<'a>(&'a mut self, id: &ItemId) -> Option<&'a mut Item> {
        self.items.get_mut(id)
    }

    async fn get_item_by_id<'a>(&'a self, id: &ItemId) -> Option<&'a Item> {
        self.items.get(id)
    }
}

#[async_trait]
impl CompleteCalendar for CachedCalendar {
    /// Returns the list of items that this calendar contains
    async fn get_items(&self) -> Result<HashMap<ItemId, &Item>, Box<dyn Error>> {
        Ok(self.items.iter()
            .map(|(id, item)| (id.clone(), item))
            .collect()
        )
    }
}
