use crate::data::Task;
use crate::data::task::TaskId;

/// A Caldav Calendar
pub struct Calendar {
    name: String,

    tasks: Vec<Task>,
}

impl Calendar {
    pub fn name(&self) -> String {
        self.name
    }

    pub fn tasks(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .map(|t| &t)
            .collect()
    }
}
