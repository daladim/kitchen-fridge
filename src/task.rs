use chrono::{Utc, DateTime};

use serde::{Deserialize, Serialize};

// TODO: turn into this one day
//      pub type TaskId = String; // This is an HTML "etag"
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskId {
    content: String,
}
impl TaskId{
    pub fn new() -> Self {
        let u = uuid::Uuid::new_v4().to_hyphenated().to_string();
        Self { content:u }
    }
}

/// A to-do task
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    id: TaskId,
    name: String,
    last_modified: DateTime<Utc>,
    completed: bool,
}

impl Task {
    /// Create a new Task
    pub fn new(name: String, last_modified: DateTime<Utc>) -> Self {
        Self {
            name, last_modified,
            id: TaskId::new(),
            completed: false
        }
    }

    pub fn id(&self) -> &TaskId     { &self.id          }
    pub fn name(&self) -> &str      { &self.name        }
    pub fn completed(&self) -> bool { self.completed    }
    pub fn last_modified(&self) -> DateTime<Utc> { self.last_modified }

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
