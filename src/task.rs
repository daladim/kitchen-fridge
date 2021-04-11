use serde::{Deserialize, Serialize};

use crate::item::ItemId;
use crate::item::SyncStatus;
use crate::calendar::CalendarId;

/// A to-do task
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    /// The task unique ID, that will never change
    id: ItemId,

    /// The sync status of this item
    sync_status: SyncStatus,

    /// The display name of the task
    name: String,
    /// The completion of the task
    completed: bool,
}

impl Task {
    /// Create a brand new Task that is not on a server yet.
    /// This will pick a new (random) task ID.
    pub fn new(name: String, completed: bool, parent_calendar_id: &CalendarId) -> Self {
        let new_item_id = ItemId::random(parent_calendar_id);
        let new_sync_status = SyncStatus::NotSynced;
        Self::new_with_parameters(name, completed, new_item_id, new_sync_status)
    }

    /// Create a new Task instance, that may be synced already
    pub fn new_with_parameters(name: String, completed: bool, id: ItemId, sync_status: SyncStatus) -> Self {
        Self {
            id,
            name,
            sync_status,
            completed,
        }
    }

    pub fn id(&self) -> &ItemId     { &self.id          }
    pub fn name(&self) -> &str      { &self.name        }
    pub fn completed(&self) -> bool { self.completed    }
    pub fn sync_status(&self) -> &SyncStatus     { &self.sync_status  }

    pub fn has_same_observable_content_as(&self, other: &Task) -> bool {
           self.id == other.id
        && self.name == other.name
        && self.completed == other.completed
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

    /// Rename a task.
    /// This updates its "last modified" field
    pub fn set_name(&mut self, new_name: String) {
        self.update_sync_status();
        self.name = new_name;
    }
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    /// Rename a task, but forces a "master" SyncStatus, just like CalDAV servers are always "masters"
    pub fn mock_remote_calendar_set_name(&mut self, new_name: String) {
        self.sync_status = SyncStatus::random_synced();
        self.name = new_name;
    }

    /// Set the completion status
    pub fn set_completed(&mut self, new_value: bool) {
        self.update_sync_status();
        self.completed = new_value;
    }
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    /// Set the completion status, but forces a "master" SyncStatus, just like CalDAV servers are always "masters"
    pub fn mock_remote_calendar_set_completed(&mut self, new_value: bool) {
        self.sync_status = SyncStatus::random_synced();
        self.completed = new_value;
    }
}
