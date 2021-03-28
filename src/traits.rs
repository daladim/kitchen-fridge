use std::error::Error;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::item::Item;
use crate::item::ItemId;
use crate::item::VersionTag;
use crate::calendar::CalendarId;

#[async_trait]
pub trait CalDavSource<T: BaseCalendar> {
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars(&self) -> Result<HashMap<CalendarId, Arc<Mutex<T>>>, Box<dyn Error>>;
    /// Returns the calendar matching the ID
    async fn get_calendar(&self, id: &CalendarId) -> Option<Arc<Mutex<T>>>;
}

/// This trait contains functions that are common to all calendars
#[async_trait]
pub trait BaseCalendar {
    /// Returns the calendar name
    fn name(&self) -> &str;

    /// Returns the calendar unique ID
    fn id(&self) -> &CalendarId;

    /// Returns the supported kinds of components for this calendar
    fn supported_components(&self) -> crate::calendar::SupportedComponents;

    /// Add an item into this calendar
    async fn add_item(&mut self, item: Item) -> Result<(), Box<dyn Error>>;

    /// Returns a particular item
    async fn get_item_by_id<'a>(&'a self, id: &ItemId) -> Option<&'a Item>;


    /// Returns whether this calDAV calendar supports to-do items
    fn supports_todo(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::TODO)
    }

    /// Returns whether this calDAV calendar supports calendar items
    fn supports_events(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::EVENT)
    }
}


/// Functions availabe for calendars that are backed by a CalDAV server
#[async_trait]
pub trait DavCalendar : BaseCalendar {
    /// Get the IDs and the version tags of every item in this calendar
    async fn get_item_version_tags(&self) -> Result<HashMap<ItemId, VersionTag>, Box<dyn Error>>;

    /// Delete an item
    async fn delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>>;

    /// Get the IDs of all current items in this calendar
    async fn get_item_ids(&self) -> Result<HashSet<ItemId>, Box<dyn Error>> {
        let items = self.get_item_version_tags().await?;
        Ok(items.iter()
            .map(|(id, _tag)| id.clone())
            .collect())
    }
}


/// Functions availabe for calendars we have full knowledge of
///
/// Usually, these are local calendars fully backed by a local folder
#[async_trait]
pub trait CompleteCalendar : BaseCalendar {
    /// Get the IDs of all current items in this calendar
    async fn get_item_ids(&self) -> Result<HashSet<ItemId>, Box<dyn Error>>;

    /// Returns all items that this calendar contains
    async fn get_items(&self) -> Result<HashMap<ItemId, &Item>, Box<dyn Error>>;

    /// Returns a particular item
    async fn get_item_by_id_mut<'a>(&'a mut self, id: &ItemId) -> Option<&'a mut Item>;

    /// Mark an item for deletion.
    /// This is required so that the upcoming sync will know it should also also delete this task from the server
    /// (and then call [`immediately_delete_item`] once it has been successfully deleted on the server)
    async fn mark_for_deletion(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>>;

    /// Immediately remove an item. See [`mark_for_deletion`]
    async fn immediately_delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>>;
}
