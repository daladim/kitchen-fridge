//! Traits used by multiple structs in this crate

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use csscolorparser::Color;
use url::Url;

use crate::calendar::SupportedComponents;
use crate::item::Item;
use crate::item::SyncStatus;
use crate::item::VersionTag;
use crate::resource::Resource;

/// This trait must be implemented by data sources (either local caches or remote CalDAV clients)
///
/// Note that some concrete types (e.g. [`crate::cache::Cache`]) can also provide non-async versions of these functions
#[async_trait]
pub trait CalDavSource<T: BaseCalendar> {
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars(&self) -> Result<HashMap<Url, Arc<Mutex<T>>>, Box<dyn Error>>;
    /// Returns the calendar matching the URL
    async fn get_calendar(&self, url: &Url) -> Option<Arc<Mutex<T>>>;
    /// Create a calendar if it did not exist, and return it
    async fn create_calendar(
        &mut self,
        url: Url,
        name: String,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> Result<Arc<Mutex<T>>, Box<dyn Error>>;

    // Removing a calendar is not supported yet
}

/// This trait contains functions that are common to all calendars
///
/// Note that some concrete types (e.g. [`crate::calendar::cached_calendar::CachedCalendar`]) can also provide non-async versions of these functions
#[async_trait]
pub trait BaseCalendar {
    /// Returns the calendar name
    fn name(&self) -> &str;

    /// Returns the calendar URL
    fn url(&self) -> &Url;

    /// Returns the supported kinds of components for this calendar
    fn supported_components(&self) -> crate::calendar::SupportedComponents;

    /// Returns the user-defined color of this calendar
    fn color(&self) -> Option<&Color>;

    /// Add an item into this calendar, and return its new sync status.
    /// For local calendars, the sync status is not modified.
    /// For remote calendars, the sync status is updated by the server
    async fn add_item(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>>;

    /// Update an item that already exists in this calendar and returns its new `SyncStatus`
    /// This replaces a given item at a given URL
    async fn update_item(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>>;

    /// Returns whether this calDAV calendar supports to-do items
    fn supports_todo(&self) -> bool {
        self.supported_components()
            .contains(crate::calendar::SupportedComponents::TODO)
    }

    /// Returns whether this calDAV calendar supports calendar items
    fn supports_events(&self) -> bool {
        self.supported_components()
            .contains(crate::calendar::SupportedComponents::EVENT)
    }
}

/// Functions availabe for calendars that are backed by a CalDAV server
///
/// Note that some concrete types (e.g. [`crate::calendar::cached_calendar::CachedCalendar`]) can also provide non-async versions of these functions
#[async_trait]
pub trait DavCalendar: BaseCalendar {
    /// Create a new calendar
    fn new(
        name: String,
        resource: Resource,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> Self;

    /// Get the URLs and the version tags of every item in this calendar
    async fn get_item_version_tags(&self) -> Result<HashMap<Url, VersionTag>, Box<dyn Error>>;

    /// Returns a particular item
    async fn get_item_by_url(&self, url: &Url) -> Result<Option<Item>, Box<dyn Error>>;

    /// Returns a set of items.
    /// This is usually faster than calling multiple consecutive [`DavCalendar::get_item_by_url`], since it only issues one HTTP request.
    async fn get_items_by_url(&self, urls: &[Url]) -> Result<Vec<Option<Item>>, Box<dyn Error>>;

    /// Delete an item
    async fn delete_item(&mut self, item_url: &Url) -> Result<(), Box<dyn Error>>;

    /// Get the URLs of all current items in this calendar
    async fn get_item_urls(&self) -> Result<HashSet<Url>, Box<dyn Error>> {
        let items = self.get_item_version_tags().await?;
        Ok(items.iter().map(|(url, _tag)| url.clone()).collect())
    }

    // Note: the CalDAV protocol could also enable to do this:
    // fn get_current_version(&self) -> CTag
}

/// Functions availabe for calendars we have full knowledge of
///
/// Usually, these are local calendars fully backed by a local folder
///
/// Note that some concrete types (e.g. [`crate::calendar::cached_calendar::CachedCalendar`]) can also provide non-async versions of these functions
#[async_trait]
pub trait CompleteCalendar: BaseCalendar {
    /// Create a new calendar
    fn new(
        name: String,
        url: Url,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> Self;

    /// Get the URLs of all current items in this calendar
    async fn get_item_urls(&self) -> Result<HashSet<Url>, Box<dyn Error>>;

    /// Returns all items that this calendar contains
    async fn get_items(&self) -> Result<HashMap<Url, &Item>, Box<dyn Error>>;

    /// Returns all items that this calendar contains
    async fn get_items_mut(&mut self) -> Result<HashMap<Url, &mut Item>, Box<dyn Error>>;

    /// Returns a particular item
    async fn get_item_by_url<'a>(&'a self, url: &Url) -> Option<&'a Item>;

    /// Returns a particular item
    async fn get_item_by_url_mut<'a>(&'a mut self, url: &Url) -> Option<&'a mut Item>;

    /// Mark an item for deletion.
    /// This is required so that the upcoming sync will know it should also also delete this task from the server
    /// (and then call [`CompleteCalendar::immediately_delete_item`] once it has been successfully deleted on the server)
    async fn mark_for_deletion(&mut self, item_id: &Url) -> Result<(), Box<dyn Error>>;

    /// Immediately remove an item. See [`CompleteCalendar::mark_for_deletion`]
    async fn immediately_delete_item(&mut self, item_id: &Url) -> Result<(), Box<dyn Error>>;
}
