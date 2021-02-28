//! Calendar events

use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};

use crate::item::ItemId;

/// TODO: implement Event one day.
/// This crate currently only supports tasks, not calendar events.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    id: ItemId,
    name: String,
    last_modified: DateTime<Utc>,
}

impl Event {
    pub fn id(&self) -> &ItemId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn last_modified(&self) -> DateTime<Utc> {
        self.last_modified
    }
}
