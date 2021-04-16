use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::item::ItemId;
use crate::item::SyncStatus;
use crate::calendar::CalendarId;

/// A to-do task
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    /// The task URL
    id: ItemId,

    /// Persistent, globally unique identifier for the calendar component
    /// The [RFC](https://tools.ietf.org/html/rfc5545#page-117) recommends concatenating a timestamp with the server's domain name, but UUID are even better
    uid: String,

    /// The sync status of this item
    sync_status: SyncStatus,
    /// The time this item was created.
    /// This is not required by RFC5545. This will be populated in tasks created by this crate, but can be None for tasks coming from a server
    creation_date: Option<DateTime<Utc>>,
    /// The last time this item was modified
    last_modified: DateTime<Utc>,

    /// The display name of the task
    name: String,
    /// The time it was completed. Set to None if this task has not been completed
    completion_date: Option<DateTime<Utc>>,
}

impl Task {
    /// Create a brand new Task that is not on a server yet.
    /// This will pick a new (random) task ID.
    pub fn new(name: String, completed: bool, parent_calendar_id: &CalendarId) -> Self {
        let new_item_id = ItemId::random(parent_calendar_id);
        let new_sync_status = SyncStatus::NotSynced;
        let new_uid = Uuid::new_v4().to_hyphenated().to_string();
        let new_creation_date = Some(Utc::now());
        let new_last_modified = Utc::now();
        let new_completion_date = if completed { Some(Utc::now()) } else { None };
        Self::new_with_parameters(name, new_uid, new_item_id, new_sync_status, new_creation_date, new_last_modified, new_completion_date)
    }

    /// Create a new Task instance, that may be synced already
    pub fn new_with_parameters(name: String, uid: String, id: ItemId,
                               sync_status: SyncStatus, creation_date: Option<DateTime<Utc>>, last_modified: DateTime<Utc>, completion_date: Option<DateTime<Utc>>) -> Self
    {
        Self {
            id,
            uid,
            name,
            sync_status,
            completion_date,
            creation_date,
            last_modified,
        }
    }

    pub fn id(&self) -> &ItemId     { &self.id          }
    pub fn uid(&self) -> &str       { &self.uid         }
    pub fn name(&self) -> &str      { &self.name        }
    pub fn completed(&self) -> bool { self.completion_date.is_some() }
    pub fn sync_status(&self) -> &SyncStatus      { &self.sync_status  }
    pub fn last_modified(&self) -> &DateTime<Utc> { &self.last_modified }
    pub fn creation_date(&self) -> Option<&DateTime<Utc>>   { self.creation_date.as_ref() }
    pub fn completion_date(&self) -> Option<&DateTime<Utc>> { self.completion_date.as_ref() }

    pub fn has_same_observable_content_as(&self, other: &Task) -> bool {
           self.id == other.id
        && self.name == other.name
        && self.completion_date == other.completion_date
        && self.last_modified == other.last_modified
        // sync status must be the same variant, but we ignore its embedded version tag
        && std::mem::discriminant(&self.sync_status) == std::mem::discriminant(&other.sync_status)
    }

    pub fn set_sync_status(&mut self, new_status: SyncStatus) {
        self.sync_status = new_status;
    }

    fn update_sync_status(&mut self) {
        match &self.sync_status {
            SyncStatus::NotSynced => return,
            SyncStatus::LocallyModified(_) => return,
            SyncStatus::Synced(prev_vt) => {
                self.sync_status = SyncStatus::LocallyModified(prev_vt.clone());
            }
            SyncStatus::LocallyDeleted(_) => {
                log::warn!("Trying to update an item that has previously been deleted. These changes will probably be ignored at next sync.");
                return;
            },
        }
    }

    fn update_last_modified(&mut self) {
        self.last_modified = Utc::now();
    }


    /// Rename a task.
    /// This updates its "last modified" field
    pub fn set_name(&mut self, new_name: String) {
        self.update_sync_status();
        self.update_last_modified();
        self.name = new_name;
    }
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    /// Rename a task, but forces a "master" SyncStatus, just like CalDAV servers are always "masters"
    pub fn mock_remote_calendar_set_name(&mut self, new_name: String) {
        self.sync_status = SyncStatus::random_synced();
        self.name = new_name;
    }

    /// Set the completion date (or pass None to un-complete the task)
    pub fn set_completed_on(&mut self, new_completion_date: Option<DateTime<Utc>>) {
        self.update_sync_status();
        self.update_last_modified();
        self.completion_date = new_completion_date;
    }
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    /// Set the completion status, but forces a "master" SyncStatus, just like CalDAV servers are always "masters"
    pub fn mock_remote_calendar_set_completed_on(&mut self, nnew_completion_date: Option<DateTime<Utc>>) {
        self.sync_status = SyncStatus::random_synced();
        self.completion_date = new_completion_date;
    }
}
