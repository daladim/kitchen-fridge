use std::convert::TryFrom;
use std::error::Error;

use url::Url;
use serde::{Deserialize, Serialize};

use crate::task::Task;
use crate::task::TaskId;

use bitflags::bitflags;

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct SupportedComponents: u8 {
        /// An event, such as a calendar meeting
        const EVENT = 1;
        /// A to-do item, such as a reminder
        const TODO = 2;
    }
}

impl TryFrom<minidom::Element> for SupportedComponents {
    type Error = Box<dyn Error>;

    /// Create an instance from an XML <supported-calendar-component-set> element
    fn try_from(element: minidom::Element) -> Result<Self, Self::Error> {
        if element.name() != "supported-calendar-component-set" {
            return Err("Element must be a <supported-calendar-component-set>".into());
        }

        let mut flags = Self::empty();
        for child in element.children() {
            match child.attr("name") {
                None => continue,
                Some("VEVENT") => flags.insert(Self::EVENT),
                Some("VTODO") => flags.insert(Self::TODO),
                Some(other) => {
                    log::warn!("Unimplemented supported component type: {:?}. Ignoring it", other);
                    continue
                },
            };
        }

        Ok(flags)
    }
}


/// A Caldav Calendar
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Calendar {
    name: String,
    url: Url,
    supported_components: SupportedComponents,

    tasks: Vec<Task>,
}

impl Calendar {
    /// Create a new calendar
    pub fn new(name: String, url: Url, supported_components: SupportedComponents) -> Self {
        Self {
            name, url, supported_components,
            tasks: Vec::new(),
        }
    }

    /// Returns the calendar name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the calendar URL
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns whether this calDAV calendar supports to-do items
    pub fn supports_todo(&self) -> bool {
        self.supported_components.contains(SupportedComponents::TODO)
    }

    /// Add a task into this calendar
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn delete_task(&mut self, task_id: &TaskId) {
        self.tasks.retain(|t| t.id() != task_id);
    }

    /// Returns the list of tasks that this calendar contains
    /// Pass a `completed` flag to filter only the completed (or non-completed) tasks
    pub fn get_tasks(&self, completed: Option<bool>) -> Vec<&Task> {
        self.get_tasks_modified_since(None, completed)
    }

    /// Returns a particular task
    pub fn get_task_by_id_mut(&mut self, id: &TaskId) -> Option<&mut Task> {
        for task in &mut self.tasks {
            if task.id() == id {
                return Some(task);
            }
        }
        return None;
    }


    /// Returns the tasks that have been last-modified after `since`
    /// Pass a `completed` flag to filter only the completed (or non-completed) tasks
    fn get_tasks_modified_since(&self, _since: Option<std::time::SystemTime>, _completed: Option<bool>) -> Vec<&Task> {
        self.tasks
            .iter()
            .collect()
    }
}
