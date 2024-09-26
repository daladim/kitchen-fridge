use serde::{Deserialize, Serialize};

/// Describes whether this item has been synced already, or modified since the last time it was synced
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum SyncStatus {
    /// This item has been locally created, and never synced yet
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

    pub fn symbol(&self) -> char {
        match self {
            SyncStatus::NotSynced => '.',
            SyncStatus::Synced(_) => '=',
            SyncStatus::LocallyModified(_) => '~',
            SyncStatus::LocallyDeleted(_) => 'x',
        }
    }
}
impl Default for SyncStatus {
    /// The default sync status is NotSynced
    fn default() -> Self {
        Self::NotSynced
    }
}
impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSynced => write!(f, "NotSynced"),
            Self::Synced(vt) => write!(f, "Synced({})", vt.tag),
            Self::LocallyModified(vt) => write!(f, "LocallyModified({})", vt.tag),
            Self::LocallyDeleted(vt) => write!(f, "LocallyDeleted({})", vt.tag),
        }
    }
}

pub trait Syncable {
    /// The value being synced
    fn value(&self) -> &String;

    fn sync_status(&self) -> &SyncStatus;

    fn set_sync_status(&mut self, new_status: SyncStatus);

    fn mark_modified_since_last_sync(&mut self) {
        match self.sync_status() {
            SyncStatus::NotSynced | SyncStatus::LocallyModified(_) => { /* do nothing */ }
            SyncStatus::Synced(prev_vt) => {
                self.set_sync_status(SyncStatus::LocallyModified(prev_vt.clone()));
            }
            SyncStatus::LocallyDeleted(_) => {
                log::warn!("Trying to update an item that has previously been deleted. These changes will probably be ignored at next sync.");
            }
        }
    }

    fn mark_synced(&mut self, synced_to: VersionTag) {
        self.set_sync_status(SyncStatus::Synced(synced_to));
    }
}

/// A VersionTag is basically a CalDAV `ctag` or `etag`. Whenever it changes, this means the data has changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
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
