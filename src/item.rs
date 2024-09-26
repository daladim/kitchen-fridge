//! CalDAV items (todo, events, journals...)
// TODO: move Event and Task to nest them in crate::items::calendar::Calendar?

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::utils::sync::{SyncStatus, Syncable};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ItemType {
    Calendar,
    Event,
    Task,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Item {
    Event(crate::event::Event),
    Task(crate::task::Task),
}

/// Returns `task.$property_name` or `event.$property_name`, depending on whether self is a Task or an Event
macro_rules! synthetise_common_getter {
    ($property_name:ident, $return_type:ty) => {
        pub fn $property_name(&self) -> $return_type {
            match self {
                Item::Event(e) => e.$property_name(),
                Item::Task(t) => t.$property_name(),
            }
        }
    };
}

impl Item {
    synthetise_common_getter!(url, &Url);
    synthetise_common_getter!(uid, &str);
    synthetise_common_getter!(name, &str);
    synthetise_common_getter!(creation_date, Option<&DateTime<Utc>>);
    synthetise_common_getter!(last_modified, &DateTime<Utc>);
    synthetise_common_getter!(sync_status, &SyncStatus);
    synthetise_common_getter!(ical_prod_id, &str);

    pub fn set_sync_status(&mut self, new_status: SyncStatus) {
        match self {
            Item::Event(e) => e.set_sync_status(new_status),
            Item::Task(t) => t.set_sync_status(new_status),
        }
    }

    pub fn is_event(&self) -> bool {
        matches!(self, Item::Event(_))
    }

    pub fn is_task(&self) -> bool {
        matches!(self, Item::Task(_))
    }

    /// Returns a mutable reference to the inner Task
    ///
    /// # Panics
    /// Panics if the inner item is not a Task
    pub fn unwrap_task_mut(&mut self) -> &mut crate::task::Task {
        match self {
            Item::Task(t) => t,
            _ => panic!("Not a task"),
        }
    }

    /// Returns a reference to the inner Task
    ///
    /// # Panics
    /// Panics if the inner item is not a Task
    pub fn unwrap_task(&self) -> &crate::task::Task {
        match self {
            Item::Task(t) => t,
            _ => panic!("Not a task"),
        }
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn has_same_observable_content_as(&self, other: &Item) -> bool {
        match (self, other) {
            (Item::Event(s), Item::Event(o)) => s.has_same_observable_content_as(o),
            (Item::Task(s), Item::Task(o)) => s.has_same_observable_content_as(o),
            _ => false,
        }
    }

    pub fn type_(&self) -> ItemType {
        match self {
            Self::Event(_) => ItemType::Event,
            Self::Task(_) => ItemType::Task,
        }
    }
}
