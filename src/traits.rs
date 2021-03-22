use std::error::Error;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::item::Item;
use crate::item::ItemId;
use crate::item::VersionTag;
use crate::calendar::CalendarId;

#[async_trait]
pub trait CalDavSource<T: PartialCalendar> {
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars(&self) -> Result<HashMap<CalendarId, Arc<Mutex<T>>>, Box<dyn Error>>;
    /// Returns the calendar matching the ID
    async fn get_calendar(&self, id: &CalendarId) -> Option<Arc<Mutex<T>>>;
}

/// A calendar we have a partial knowledge of.
///
/// Usually, this is a calendar from a remote source, that is synced to a CompleteCalendar
#[async_trait]
pub trait PartialCalendar {
    /// Returns the calendar name
    fn name(&self) -> &str;

    /// Returns the calendar unique ID
    fn id(&self) -> &CalendarId;

    /// Returns the supported kinds of components for this calendar
    fn supported_components(&self) -> crate::calendar::SupportedComponents;

    /// Get the IDs and the version tags of every item in this calendar
    async fn get_item_version_tags(&self) -> Result<HashMap<ItemId, VersionTag>, Box<dyn Error>>;

    /// Returns a particular item
    async fn get_item_by_id_mut<'a>(&'a mut self, id: &ItemId) -> Option<&'a mut Item>;

    /// Returns a particular item
    async fn get_item_by_id<'a>(&'a self, id: &ItemId) -> Option<&'a Item>;

    /// Add an item into this calendar
    async fn add_item(&mut self, item: Item);

    /// Remove an item from this calendar
    async fn delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>>;


    /// Returns whether this calDAV calendar supports to-do items
    fn supports_todo(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::TODO)
    }

    /// Returns whether this calDAV calendar supports calendar items
    fn supports_events(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::EVENT)
    }

    /// Get the IDs of all current items in this calendar
    async fn get_item_ids(&self) -> Result<HashSet<ItemId>, Box<dyn Error>> {
        let items = self.get_item_version_tags().await?;
        Ok(items.iter()
            .map(|(id, _tag)| id.clone())
            .collect())
    }

    /// Finds the IDs of the items that are missing compared to a reference set
    async fn find_deletions_from(&self, reference_set: HashSet<ItemId>) -> Result<HashSet<ItemId>, Box<dyn Error>> {
        let current_items = self.get_item_ids().await?;
        Ok(reference_set.difference(&current_items).map(|id| id.clone()).collect())
    }
}

/// A calendar we always know everything about.
///
/// Usually, this is a calendar fully stored on a local disk
#[async_trait]
pub trait CompleteCalendar : PartialCalendar {
    /// Returns the list of items that this calendar contains
    async fn get_items(&self) -> Result<HashMap<ItemId, &Item>, Box<dyn Error>>;
}

