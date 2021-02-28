use std::convert::TryFrom;
use std::error::Error;
use std::collections::HashMap;
use std::collections::BTreeMap;

use url::Url;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
    deleted_tasks: BTreeMap<DateTime<Utc>, TaskId>,
}

impl Calendar {
    /// Create a new calendar
    pub fn new(name: String, url: Url, supported_components: SupportedComponents) -> Self {
        Self {
            name, url, supported_components,
            tasks: Vec::new(),
            deleted_tasks: BTreeMap::new(),
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
        self.deleted_tasks.insert(Utc::now(), task_id.clone());
    }

    /// Returns the list of tasks that this calendar contains
    /// Pass a `completed` flag to filter only the completed (or non-completed) tasks
    pub fn get_tasks(&self, completed: Option<bool>) -> HashMap<TaskId, &Task> {
        self.get_tasks_modified_since(None, completed)
    }

    /// Returns the tasks that have been last-modified after `since`
    /// Pass a `completed` flag to filter only the completed (or non-completed) tasks
    pub fn get_tasks_modified_since(&self, since: Option<DateTime<Utc>>, _completed: Option<bool>) -> HashMap<TaskId, &Task> {
        let mut map = HashMap::new();

        for task in &self.tasks {
            match since {
                None => (),
                Some(since) => if task.last_modified() < since {
                    continue;
                },
            }

            map.insert(task.id().clone(), task);
        }

        map
    }

    /// Returns the tasks that have been deleted after `since`
    pub fn get_tasks_deleted_since(&self, since: DateTime<Utc>) -> Vec<TaskId> {
        self.deleted_tasks.range(since..)
            .map(|(_key, value)| value.clone())
            .collect()
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
}
