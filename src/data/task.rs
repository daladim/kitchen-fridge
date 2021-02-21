pub type TaskId = String; // This is an HTML "etag"

/// A to-do task
#[derive(Clone, Debug)]
pub struct Task {
    id: TaskId,
    name: String,
    completed: bool,
}

impl Task {
    pub fn id(&self) -> &TaskId     { &self.id          }
    pub fn name(&self) -> &str      { &self.name        }
    pub fn completed(&self) -> bool { self.completed    }

    pub fn set_completed(&mut self) {
        // TODO: either require a reference to the DataSource, so that it is aware
        //       or change a flag here, and the DataSource will be able to check the flags of all its content (but then the Calendar should only give a reference/Arc, not a clone)
    }
}