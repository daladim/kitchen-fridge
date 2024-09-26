//! Calendar events (iCal `VEVENT` items)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::item::SyncStatus;

/// TODO: implement `Event` one day.
/// This crate currently only supports tasks, not calendar events.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    uid: String,
    name: String,
    sync_status: SyncStatus,
}

impl Event {
    pub fn new() -> Self {
        unimplemented!();
    }

    pub fn url(&self) -> &Url {
        unimplemented!();
    }

    pub fn uid(&self) -> &str {
        &self.uid
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ical_prod_id(&self) -> &str {
        unimplemented!()
    }

    pub fn creation_date(&self) -> Option<&DateTime<Utc>> {
        unimplemented!()
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

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn has_same_observable_content_as(&self, _other: &Event) -> bool {
        unimplemented!();
    }
}
