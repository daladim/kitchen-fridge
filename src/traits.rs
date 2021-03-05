use std::error::Error;
use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::item::Item;
use crate::item::ItemId;
use crate::calendar::CalendarId;

#[async_trait]
pub trait CalDavSource<T: PartialCalendar> {
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars(&self) -> Result<&HashMap<CalendarId, T>, Box<dyn Error>>;
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars_mut(&mut self) -> Result<HashMap<CalendarId, &mut T>, Box<dyn Error>>;

    //
    //
    // TODO: find a better search key (do calendars have a unique ID?)
    // TODO: search key should be a reference
    //
    /// Returns the calendar matching the ID
    async fn get_calendar(&self, id: CalendarId) -> Option<&T>;
    /// Returns the calendar matching the ID
    async fn get_calendar_mut(&mut self, id: CalendarId) -> Option<&mut T>;

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

    /// Returns the calendar unique ID
    fn id(&self) -> &CalendarId;

    /// Returns the supported kinds of components for this calendar
    fn supported_components(&self) -> crate::calendar::SupportedComponents;

    /// Returns the items that have been last-modified after `since`
    fn get_items_modified_since(&self, since: Option<DateTime<Utc>>, filter: Option<crate::calendar::SearchFilter>)
        -> HashMap<ItemId, &Item>;

    /// Get the IDs of all current items in this calendar
    fn get_item_ids(&mut self) -> HashSet<ItemId>;

    /// Returns a particular item
    fn get_item_by_id_mut(&mut self, id: &ItemId) -> Option<&mut Item>;

    /// Add an item into this calendar
    fn add_item(&mut self, item: Item);

    /// Remove an item from this calendar
    fn delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>>;


    /// Returns whether this calDAV calendar supports to-do items
    fn supports_todo(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::TODO)
    }

    /// Returns whether this calDAV calendar supports calendar items
    fn supports_events(&self) -> bool {
        self.supported_components().contains(crate::calendar::SupportedComponents::EVENT)
    }

    /// Finds the IDs of the items that are missing compared to a reference set
    fn find_deletions_from(&mut self, reference_set: HashSet<ItemId>) -> HashSet<ItemId> {
        let current_items = self.get_item_ids();
        reference_set.difference(&current_items).map(|id| id.clone()).collect()
    }
}

/// A calendar we always know everything about.
///
/// Usually, this is a calendar fully stored on a local disk
pub trait CompleteCalendar : PartialCalendar {
    /// Returns the items that have been deleted after `since`
    ///
    /// See also [`PartialCalendar::get_items_deleted_since`]
    fn get_items_deleted_since(&self, since: DateTime<Utc>) -> HashSet<ItemId>;

    /// Returns the list of items that this calendar contains
    fn get_items(&self) -> HashMap<ItemId, &Item>;
}

