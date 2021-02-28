use std::convert::TryFrom;
use std::error::Error;
use std::collections::HashMap;
use std::collections::BTreeMap;

use url::Url;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::Item;
use crate::item::ItemId;

use bitflags::bitflags;

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct SupportedComponents: u8 {
        /// An event, such as a calendar meeting
        const EVENT = 1;
        /// A to-do item, such as a reminder
        const TODO = 2;
    }
}

impl TryFrom<minidom::Element> for SupportedComponents {
    type Error = Box<dyn Error>;

    /// Create an instance from an XML <supported-calendar-component-set> element
    fn try_from(element: minidom::Element) -> Result<Self, Self::Error> {
        if element.name() != "supported-calendar-component-set" {
            return Err("Element must be a <supported-calendar-component-set>".into());
        }

        let mut flags = Self::empty();
        for child in element.children() {
            match child.attr("name") {
                None => continue,
                Some("VEVENT") => flags.insert(Self::EVENT),
                Some("VTODO") => flags.insert(Self::TODO),
                Some(other) => {
                    log::warn!("Unimplemented supported component type: {:?}. Ignoring it", other);
                    continue
                },
            };
        }

        Ok(flags)
    }
}


/// Flags to tell which events should be retrieved
pub enum SearchFilter {
    /// Return all items
    All,
    /// Return only tasks
    Tasks,
    // /// Return only completed tasks
    // CompletedTasks,
    // /// Return only calendar events
    // Events,
}

impl Default for SearchFilter {
    fn default() -> Self {
        SearchFilter::All
    }
}

/// A Caldav Calendar
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Calendar {
    name: String,
    url: Url,
    supported_components: SupportedComponents,

    items: Vec<Item>,
    deleted_items: BTreeMap<DateTime<Utc>, ItemId>,
}

impl Calendar {
    /// Create a new calendar
    pub fn new(name: String, url: Url, supported_components: SupportedComponents) -> Self {
        Self {
            name, url, supported_components,
            items: Vec::new(),
            deleted_items: BTreeMap::new(),
        }
    }

    /// Returns the calendar name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the calendar URL
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns whether this calDAV calendar supports to-do items
    pub fn supports_todo(&self) -> bool {
        self.supported_components.contains(SupportedComponents::TODO)
    }

    /// Returns whether this calDAV calendar supports calendar items
    pub fn supports_events(&self) -> bool {
        self.supported_components.contains(SupportedComponents::EVENT)
    }

    /// Add an item into this calendar
    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Remove an item from this calendar
    pub fn delete_item(&mut self, item_id: &ItemId) {
        self.items.retain(|i| i.id() != item_id);
        self.deleted_items.insert(Utc::now(), item_id.clone());
    }

    /// Returns the list of items that this calendar contains
    pub fn get_items(&self) -> HashMap<ItemId, &Item> {
        self.get_items_modified_since(None, None)
    }
    /// Returns the items that have been last-modified after `since`
    pub fn get_items_modified_since(&self, since: Option<DateTime<Utc>>, filter: Option<SearchFilter>) -> HashMap<ItemId, &Item> {
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

    /// Returns the items that have been deleted after `since`
    pub fn get_items_deleted_since(&self, since: DateTime<Utc>) -> Vec<ItemId> {
        self.deleted_items.range(since..)
        .map(|(_key, value)| value.clone())
        .collect()
    }

    /// Returns a particular item
    pub fn get_item_by_id_mut(&mut self, id: &ItemId) -> Option<&mut Item> {
        for item in &mut self.items {
            if item.id() == id {
                return Some(item);
            }
        }
        return None;
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
