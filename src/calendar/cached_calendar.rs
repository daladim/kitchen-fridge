use std::collections::HashMap;
use std::collections::BTreeMap;

use url::Url;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::traits::{PartialCalendar, CompleteCalendar};
use crate::calendar::{SupportedComponents, SearchFilter};
use crate::Item;
use crate::item::ItemId;


/// A calendar used by the [`cache`](crate::cache) module
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CachedCalendar {
    name: String,
    url: Url,
    supported_components: SupportedComponents,

    items: Vec<Item>,
    deleted_items: BTreeMap<DateTime<Utc>, ItemId>,
}

impl CachedCalendar {
    /// Create a new calendar
    pub fn new(name: String, url: Url, supported_components: SupportedComponents) -> Self {
        Self {
            name, url, supported_components,
            items: Vec::new(),
            deleted_items: BTreeMap::new(),
        }
    }

    /// Returns the list of tasks that this calendar contains
    pub fn get_tasks(&self) -> HashMap<ItemId, &Item> {
        self.get_tasks_modified_since(None)
    }
    /// Returns the tasks that have been last-modified after `since`
    pub fn get_tasks_modified_since(&self, since: Option<DateTime<Utc>>) -> HashMap<ItemId, &Item> {
        self.get_items_modified_since(since, Some(SearchFilter::Tasks))
    }
}

impl PartialCalendar for CachedCalendar {
    fn name(&self) -> &str {
        &self.name
    }

    fn url(&self) -> &Url {
        &self.url
    }

    fn supported_components(&self) -> SupportedComponents {
        self.supported_components
    }

    fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    fn delete_item(&mut self, item_id: &ItemId) {
        self.items.retain(|i| i.id() != item_id);
        self.deleted_items.insert(Utc::now(), item_id.clone());
    }

    fn get_items_modified_since(&self, since: Option<DateTime<Utc>>, filter: Option<SearchFilter>) -> HashMap<ItemId, &Item> {
        let filter = filter.unwrap_or_default();

        let mut map = HashMap::new();

        for item in &self.items {
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

    fn get_item_by_id_mut(&mut self, id: &ItemId) -> Option<&mut Item> {
        for item in &mut self.items {
            if item.id() == id {
                return Some(item);
            }
        }
        return None;
    }

    fn find_missing_items_compared_to(&self, _other: &dyn PartialCalendar) -> Vec<ItemId> {
        unimplemented!("todo");
    }
}

impl CompleteCalendar for CachedCalendar {
    /// Returns the items that have been deleted after `since`
    fn get_items_deleted_since(&self, since: DateTime<Utc>) -> Vec<ItemId> {
        self.deleted_items.range(since..)
        .map(|(_key, value)| value.clone())
        .collect()
    }

    /// Returns the list of items that this calendar contains
    fn get_items(&self) -> HashMap<ItemId, &Item> {
        self.get_items_modified_since(None, None)
    }

}
