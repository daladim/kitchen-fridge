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
    pub fn new(name: String, url: Url, supported_components: SupportedComponents) -> Self {
        Self {
            name, url, supported_components,
            tasks: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns whether this calDAV calendar supports to-do items
    pub fn supports_todo(&self) -> bool {
        self.supported_components.contains(SupportedComponents::TODO)
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
