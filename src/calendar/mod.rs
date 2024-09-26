//! Various objects that implement Calendar-related traits

pub mod cached_calendar;
pub mod remote_calendar;

use std::convert::TryFrom;
use std::error::Error;

use serde::{Deserialize, Serialize};

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

impl SupportedComponents {
    pub fn to_xml_string(&self) -> String {
        format!(
            r#"
            <B:supported-calendar-component-set>
                {} {}
            </B:supported-calendar-component-set>
            "#,
            if self.contains(Self::EVENT) {
                "<B:comp name=\"VEVENT\"/>"
            } else {
                ""
            },
            if self.contains(Self::TODO) {
                "<B:comp name=\"VTODO\"/>"
            } else {
                ""
            },
        )
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
                    log::warn!(
                        "Unimplemented supported component type: {:?}. Ignoring it",
                        other
                    );
                    continue;
                }
            };
        }

        Ok(flags)
    }
}

/// Flags to tell which events should be retrieved
pub enum SearchFilter {
    /// Return all items
    All,
    /// Return only tasks
    Tasks,
    // /// Return only completed tasks
    // CompletedTasks,
    // /// Return only calendar events
    // Events,
}

impl Default for SearchFilter {
    fn default() -> Self {
        SearchFilter::All
    }
}
