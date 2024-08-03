//! CalDAV items (todo, events, journals...)
// TODO: move Event and Task to nest them in crate::items::calendar::Calendar?

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

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
        match &self {
            Item::Event(_) => true,
            _ => false,
        }
    }

    pub fn is_task(&self) -> bool {
        match &self {
            Item::Task(_) => true,
            _ => false,
        }
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
}

/// A VersionTag is basically a CalDAV `ctag` or `etag`. Whenever it changes, this means the data has changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionTag {
    tag: String,
}

impl From<String> for VersionTag {
    fn from(tag: String) -> VersionTag {
        Self { tag }
    }
}

impl VersionTag {
    /// Get the inner version tag (usually a WebDAV `ctag` or `etag`)
    pub fn as_str(&self) -> &str {
        &self.tag
    }

    /// Generate a random VersionTag
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn random() -> Self {
        let random = uuid::Uuid::new_v4().to_hyphenated().to_string();
        Self { tag: random }
    }
}

/// Describes whether this item has been synced already, or modified since the last time it was synced
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// This item has ben locally created, and never synced yet
    NotSynced,
    /// At the time this item has ben synced, it has a given version tag, and has not been locally modified since then.
    /// Note: in integration tests, in case we are mocking a remote calendar by a local calendar, this is the only valid variant (remote calendars make no distinction between all these variants)
    Synced(VersionTag),
    /// This item has been synced when it had a given version tag, and has been locally modified since then.
    LocallyModified(VersionTag),
    /// This item has been synced when it had a given version tag, and has been locally deleted since then.
    LocallyDeleted(VersionTag),
}
impl SyncStatus {
    /// Generate a random SyncStatus::Synced
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn random_synced() -> Self {
        Self::Synced(VersionTag::random())
    }
}
