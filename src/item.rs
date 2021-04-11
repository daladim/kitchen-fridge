// TODO: move Event and Task to nest them in crate::items::calendar::Calendar?

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::resource::Resource;
use crate::calendar::CalendarId;



#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Item {
    Event(crate::event::Event),
    Task(crate::task::Task),
}

impl Item {
    pub fn id(&self) -> &ItemId {
        match self {
            Item::Event(e) => e.id(),
            Item::Task(t) => t.id(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Item::Event(e) => e.name(),
            Item::Task(t) => t.name(),
        }
    }

    pub fn sync_status(&self) -> &SyncStatus {
        match self {
            Item::Event(e) => e.sync_status(),
            Item::Task(t) => t.sync_status(),
        }
    }
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

    pub fn has_same_observable_content_as(&self, other: &Item) -> bool {
        match (self, other) {
            (Item::Event(s), Item::Event(o)) => s.has_same_observable_content_as(o),
            (Item::Task(s),   Item::Task(o))   => s.has_same_observable_content_as(o),
            _ => false,
        }
    }
}


#[derive(Clone, Debug, PartialEq, Hash, Serialize, Deserialize)]
pub struct ItemId {
    content: Url,
}
impl ItemId{
    /// Generate a random ItemId.
    pub fn random(parent_calendar: &CalendarId) -> Self {
        let random = uuid::Uuid::new_v4().to_hyphenated().to_string();
        let u = parent_calendar.join(&random).unwrap(/* this cannot panic since we've just created a string that is a valid URL */);
        Self { content:u }
    }

    pub fn as_url(&self) -> &Url {
        &self.content
    }
}
impl From<Url> for ItemId {
    fn from(url: Url) -> Self {
        Self { content: url }
    }
}
impl From<&Resource> for ItemId {
    fn from(resource: &Resource) -> Self {
        Self { content: resource.url().clone() }
    }
}
impl FromStr for ItemId {
    type Err = url::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let u: Url = s.parse()?;
        Ok(Self::from(u))
    }
}

impl Eq for ItemId {}
impl Display for ItemId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.content)
    }
}


/// A VersionTag is basically a CalDAV `ctag` or `etag`. Whenever it changes, this means the data has changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionTag {
    tag: String
}

impl From<String> for VersionTag {
    fn from(tag: String) -> VersionTag {
        Self { tag }
    }
}

impl VersionTag {
    /// Generate a random VesionTag
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn random() -> Self {
        let random = uuid::Uuid::new_v4().to_hyphenated().to_string();
        Self { tag: random }
    }
}



/// Desribes whether this item has been synced already, or modified since the last time it was synced
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
