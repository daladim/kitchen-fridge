use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::resource::Resource;



#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
}


#[derive(Clone, Debug, PartialEq, Hash, Serialize, Deserialize)]
pub struct ItemId {
    content: Url,
}
impl ItemId{
    /// Generate a random ItemId. This should only be useful in tests
    pub fn random() -> Self {
        let random = uuid::Uuid::new_v4().to_hyphenated().to_string();
        let s = format!("https://server.com/{}", random);
        let u = s.parse().unwrap();
        Self { content:u }
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
        s.parse()
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
    #[cfg(feature = "mock_version_tag")]
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
    /// At the time this item has ben synced, it has a given version tag, and has not been locally modified since then
    Synced(VersionTag),
    /// This item has been synced when it had a given version tag, and has been locally modified since then.
    LocallyModified(VersionTag),
    /// This item has been synced when it had a given version tag, and has been locally deleted since then.
    LocallyDeleted(VersionTag),
}
impl SyncStatus {
    /// Generate a random SyncStatus::Synced
    #[cfg(feature = "mock_version_tag")]
    pub fn random_synced() -> Self {
        Self::Synced(VersionTag::random())
    }
}
