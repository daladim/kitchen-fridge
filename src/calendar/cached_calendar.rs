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

    /// Returns the list of tasks that this calendar contains
    pub async fn get_tasks(&self) -> HashMap<ItemId, &Item> {
        self.get_tasks_modified_since(None).await
    }
    /// Returns the tasks that have been last-modified after `since`
    pub async fn get_tasks_modified_since(&self, since: Option<DateTime<Utc>>) -> HashMap<ItemId, &Item> {
        self.get_items_modified_since(since, Some(SearchFilter::Tasks)).await
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

    async fn get_items_modified_since(&self, since: Option<DateTime<Utc>>, filter: Option<SearchFilter>) -> HashMap<ItemId, &Item> {
        let filter = filter.unwrap_or_default();

        let mut map = HashMap::new();

        for (_id, item) in &self.items {
            match since {
                None => (),
                Some(since) => if item.last_modified() < since {
                    continue;
                },
            }

            match filter {
                SearchFilter::Tasks => {
                    if item.is_task() == false {
                        continue;
                    }
                },
                _ => (),
            }

            map.insert(item.id().clone(), item);
        }

        map
    }

    async fn get_item_ids(&mut self) -> HashSet<ItemId> {
        self.items.keys().cloned().collect()
    }

    async fn get_item_by_id_mut<'a>(&'a mut self, id: &ItemId) -> Option<&'a mut Item> {
        self.items.get_mut(id)
    }
}

#[async_trait]
impl CompleteCalendar for CachedCalendar {
    /// Returns the items that have been deleted after `since`
    async fn get_items_deleted_since(&self, since: DateTime<Utc>) -> HashSet<ItemId> {
        self.deleted_items.range(since..)
            .map(|(_key, id)| id.clone())
            .collect()
    }

    /// Returns the list of items that this calendar contains
    async fn get_items(&self) -> HashMap<ItemId, &Item> {
        self.get_items_modified_since(None, None).await
    }

}
