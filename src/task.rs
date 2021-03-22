use chrono::{Utc, DateTime};
use serde::{Deserialize, Serialize};

use crate::item::ItemId;
use crate::item::VersionTag;

/// A to-do task
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// The task unique ID, that will never change
    id: ItemId,

    /// The last modification date of this task
    last_modified: DateTime<Utc>,
    /// The version tag of this item
    version_tag: VersionTag,

    /// The display name of the task
    name: String,
    /// The completion of the task
    completed: bool,
}

impl Task {
    /// Create a new Task
    pub fn new(name: String, id: ItemId, last_modified: DateTime<Utc>, version_tag: VersionTag) -> Self {
        Self {
            id,
            name,
            last_modified,
            version_tag,
            completed: false,
        }
    }

    pub fn id(&self) -> &ItemId     { &self.id          }
    pub fn name(&self) -> &str      { &self.name        }
    pub fn completed(&self) -> bool { self.completed    }
    pub fn last_modified(&self) -> DateTime<Utc> { self.last_modified }
    pub fn version_tag(&self) -> &VersionTag     { &self.version_tag  }

    fn update_last_modified(&mut self) {
        self.last_modified = Utc::now();
    }

    /// Rename a task.
    /// This updates its "last modified" field
    pub fn set_name(&mut self, new_name: String) {
        self.update_last_modified();
        self.name = new_name;
    }

    pub fn set_completed(&mut self, new_value: bool) {
        // TODO: either require a reference to the DataSource, so that it is aware
        //       or change a flag here, and the DataSource will be able to check the flags of all its content (but then the Calendar should only give a reference/Arc, not a clone)
        self.update_last_modified();
        self.completed = new_value;
    }
}
