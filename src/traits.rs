use std::error::Error;
use std::collections::HashMap;

use async_trait::async_trait;
use url::Url;
use chrono::{DateTime, Utc};

use crate::item::Item;
use crate::item::ItemId;

#[async_trait]
pub trait CalDavSource<T: PartialCalendar> {
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars(&self) -> Result<&Vec<T>, Box<dyn Error>>;
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars_mut(&mut self) -> Result<Vec<&mut T>, Box<dyn Error>>;

    //
    //
    // TODO: find a better search key (do calendars have a unique ID?)
    // TODO: search key should be a reference
    //
    /// Returns the calendar matching the URL
    async fn get_calendar(&self, url: Url) -> Option<&T>;
    /// Returns the calendar matching the URL
    async fn get_calendar_mut(&mut self, url: Url) -> Option<&mut T>;

}

pub trait SyncSlave {
    /// Returns the last time this source successfully synced from a master source (e.g. from a server)
    /// (or None in case it has never been synchronized)
    fn get_last_sync(&self) -> Option<DateTime<Utc>>;
    /// Update the last sync timestamp to now, or to a custom time in case `timepoint` is `Some`
    fn update_last_sync(&mut self, timepoint: Option<DateTime<Utc>>);
}

/// A calendar we have a partial knowledge of.
///
/// Usually, this is a calendar from a remote source, that is synced to a CompleteCalendar
pub trait PartialCalendar {
    /// Returns the calendar name
    fn name(&self) -> &str;

    /// Returns the calendar URL
    fn url(&self) -> &Url;

    /// Returns the supported kinds of components for this calendar
    fn supported_components(&self) -> crate::calendar::SupportedComponents;

    /// Returns the items that have been last-modified after `since`
    fn get_items_modified_since(&self, since: Option<DateTime<Utc>>, filter: Option<crate::calendar::SearchFilter>)
        -> HashMap<ItemId, &Item>;

    /// Get the IDs of all current items in this calendar
    fn get_item_ids(&mut self) -> Vec<ItemId>;

    /// Returns a particular item
    fn get_item_by_id_mut(&mut self, id: &ItemId) -> Option<&mut Item>;

    /// Add an item into this calendar
    fn add_item(&mut self, item: Item);

    /// Remove an item from this calendar
    fn delete_item(&mut self, item_id: &ItemId);


    /// Returns whether this calDAV calendar supports to-do items
    fn supports_todo(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::TODO)
    }

    /// Returns whether this calDAV calendar supports calendar items
    fn supports_events(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::EVENT)
    }

    /// Finds the IDs of the items that are missing compared to a reference set
    fn find_deletions(&mut self, reference_set: Vec<ItemId>) -> Vec<ItemId> {
        let mut deletions = Vec::new();

        let current_items = self.get_item_ids();
        for original_item in reference_set {
            if current_items.contains(&original_item) == false {
                deletions.push(original_item);
            }
        }
        deletions
    }
}

/// A calendar we always know everything about.
///
/// Usually, this is a calendar fully stored on a local disk
pub trait CompleteCalendar : PartialCalendar {
    /// Returns the items that have been deleted after `since`
    ///
    /// See also [`PartialCalendar::get_items_deleted_since`]
    fn get_items_deleted_since(&self, since: DateTime<Utc>) -> Vec<ItemId>;

    /// Returns the list of items that this calendar contains
    fn get_items(&self) -> HashMap<ItemId, &Item>;
}

