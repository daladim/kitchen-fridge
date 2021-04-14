//! Calendar events

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::item::ItemId;
use crate::item::SyncStatus;

/// TODO: implement Event one day.
/// This crate currently only supports tasks, not calendar events.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    id: ItemId,
    name: String,
    sync_status: SyncStatus,
}

impl Event {
    pub fn new() -> Self {
        unimplemented!();
    }

    pub fn id(&self) -> &ItemId {
        &self.id
    }

    pub fn uid(&self) -> &str {
        unimplemented!()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn last_modified(&self) -> &DateTime<Utc> {
        unimplemented!()
    }

    pub fn sync_status(&self) -> &SyncStatus {
        &self.sync_status
    }
    pub fn set_sync_status(&mut self, new_status: SyncStatus) {
        self.sync_status = new_status;
    }

    pub fn has_same_observable_content_as(&self, _other: &Event) -> bool {
        unimplemented!();
    }
}
