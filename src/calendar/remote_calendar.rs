use std::collections::{HashMap, HashSet};
use std::error::Error;

use url::Url;
use chrono::{DateTime, Utc};

use crate::traits::PartialCalendar;
use crate::calendar::SupportedComponents;
use crate::calendar::CalendarId;
use crate::item::ItemId;
use crate::item::Item;

/// A CalDAV calendar created by a [`Client`](crate::client::Client).
#[derive(Clone)]
pub struct RemoteCalendar {
    name: String,
    url: Url,
    supported_components: SupportedComponents
}

impl RemoteCalendar {
    pub fn new(name: String, url: Url, supported_components: SupportedComponents) -> Self {
        Self {
            name, url, supported_components
        }
    }
}

impl PartialCalendar for RemoteCalendar {
    fn name(&self) -> &str { &self.name }
    fn id(&self) -> &CalendarId { &self.url }
    fn supported_components(&self) -> crate::calendar::SupportedComponents {
        self.supported_components
    }

    /// Returns the items that have been last-modified after `since`
    fn get_items_modified_since(&self, _since: Option<DateTime<Utc>>, _filter: Option<crate::calendar::SearchFilter>)
        -> HashMap<ItemId, &Item>
    {
        log::error!("Not implemented");
        HashMap::new()
    }

    /// Get the IDs of all current items in this calendar
    fn get_item_ids(&mut self) -> HashSet<ItemId> {
        log::error!("Not implemented");
        HashSet::new()
    }

    /// Returns a particular item
    fn get_item_by_id_mut(&mut self, _id: &ItemId) -> Option<&mut Item> {
        log::error!("Not implemented");
        None
    }

    /// Add an item into this calendar
    fn add_item(&mut self, _item: Item) {
        log::error!("Not implemented");
    }

    /// Remove an item from this calendar
    fn delete_item(&mut self, _item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        log::error!("Not implemented");
        Ok(())
    }

}

