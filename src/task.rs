//! To-do tasks (iCal `VTODO` item)

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use ical::property::Property;

use crate::item::ItemId;
use crate::item::SyncStatus;
use crate::calendar::CalendarId;

/// RFC5545 defines the completion as several optional fields, yet some combinations make no sense.
/// This enum provides an API that forbids such impossible combinations.
///
/// * `COMPLETED` is an optional timestamp that tells whether this task is completed
/// * `STATUS` is an optional field, that can be set to `NEEDS-ACTION`, `COMPLETED`, or others.
/// Even though having a `COMPLETED` date but a `STATUS:NEEDS-ACTION` is theorically possible, it obviously makes no sense. This API ensures this cannot happen
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompletionStatus {
    Completed(Option<DateTime<Utc>>),
    Uncompleted,
}
impl CompletionStatus {
    pub fn is_completed(&self) -> bool {
        match self {
            CompletionStatus::Completed(_) => true,
            _ => false,
        }
    }
}

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
    /// The completion status of this task
    completion_status: CompletionStatus,

    /// The display name of the task
    name: String,


    /// The PRODID, as defined in iCal files
    ical_prod_id: String,

    /// Extra parameters that have not been parsed from the iCal file (because they're not supported (yet) by this crate).
    /// They are needed to serialize this item into an equivalent iCal file
    extra_parameters: Vec<Property>,
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
        let new_completion_status = if completed {
                CompletionStatus::Completed(Some(Utc::now()))
            } else { CompletionStatus::Uncompleted };
        let ical_prod_id = crate::ical::default_prod_id();
        let extra_parameters = Vec::new();
        Self::new_with_parameters(name, new_uid, new_item_id, new_completion_status, new_sync_status, new_creation_date, new_last_modified, ical_prod_id, extra_parameters)
    }

    /// Create a new Task instance, that may be synced on the server already
    pub fn new_with_parameters(name: String, uid: String, id: ItemId,
                               completion_status: CompletionStatus,
                               sync_status: SyncStatus, creation_date: Option<DateTime<Utc>>, last_modified: DateTime<Utc>,
                               ical_prod_id: String, extra_parameters: Vec<Property>,
                            ) -> Self
    {
        Self {
            id,
            uid,
            name,
            completion_status,
            sync_status,
            creation_date,
            last_modified,
            ical_prod_id,
            extra_parameters,
        }
    }

    pub fn id(&self) -> &ItemId     { &self.id          }
    pub fn uid(&self) -> &str       { &self.uid         }
    pub fn name(&self) -> &str      { &self.name        }
    pub fn completed(&self) -> bool { self.completion_status.is_completed() }
    pub fn ical_prod_id(&self) -> &str            { &self.ical_prod_id }
    pub fn sync_status(&self) -> &SyncStatus      { &self.sync_status  }
    pub fn last_modified(&self) -> &DateTime<Utc> { &self.last_modified }
    pub fn creation_date(&self) -> Option<&DateTime<Utc>>   { self.creation_date.as_ref() }
    pub fn completion_status(&self) -> &CompletionStatus    { &self.completion_status }
    pub fn extra_parameters(&self) -> &[Property]           { &self.extra_parameters }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn has_same_observable_content_as(&self, other: &Task) -> bool {
           self.id == other.id
        && self.name == other.name
        // sync status must be the same variant, but we ignore its embedded version tag
        && std::mem::discriminant(&self.sync_status) == std::mem::discriminant(&other.sync_status)
        // completion status must be the same variant, but we ignore its embedded completion date (they are not totally mocked in integration tests)
        && std::mem::discriminant(&self.completion_status) == std::mem::discriminant(&other.completion_status)
        // last modified dates are ignored (they are not totally mocked in integration tests)
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
        self.update_last_modified();
        self.name = new_name;
    }

    /// Set the completion status
    pub fn set_completion_status(&mut self, new_completion_status: CompletionStatus) {
        self.update_sync_status();
        self.update_last_modified();
        self.completion_status = new_completion_status;
    }
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    /// Set the completion status, but forces a "master" SyncStatus, just like CalDAV servers are always "masters"
    pub fn mock_remote_calendar_set_completion_status(&mut self, new_completion_status: CompletionStatus) {
        self.sync_status = SyncStatus::random_synced();
        self.completion_status = new_completion_status;
    }
}
