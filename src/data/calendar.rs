use crate::data::Task;
use crate::data::task::TaskId;

/// A Caldav Calendar
pub struct Calendar {
    name: String,

    tasks: Vec<Task>,
}

impl Calendar {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tasks(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .collect()
    }

    pub fn task_by_id_mut(&mut self, id: TaskId) -> &mut Task {
        todo!();
        &mut self.tasks[0]
    }
}
