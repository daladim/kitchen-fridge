use crate::data::TaskView;

/// A Caldav Calendar
pub struct Calendar {
    name: String,

    tasks: Vec<TaskView>,
}

impl Calendar {
    pub fn name() -> String {
        self.name
    }

    pub fn tasks() -> Vec<TaskView> {
        self.tasks
    }
}

impl Drop for Calendar {
    fn drop(&mut self) {
        // TODO: display a warning in case some TaskViews still have a refcount > 0
    }
}
