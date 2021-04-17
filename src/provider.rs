//! This modules abstracts data sources and merges them in a single virtual one

use std::error::Error;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::traits::{BaseCalendar, CalDavSource, DavCalendar};
use crate::traits::CompleteCalendar;
use crate::item::SyncStatus;
use crate::calendar::CalendarId;

/// A counter of errors that happen during a sync
struct SyncResult {
    n_errors: u32,
}
impl SyncResult {
    pub fn new() -> Self {
        Self { n_errors: 0 }
    }
    pub fn is_success(&self) -> bool {
        self.n_errors == 0
    }

    pub fn error(&mut self, text: &str) {
        log::error!("{}", text);
        self.n_errors += 1;
    }
    pub fn warn(&mut self, text: &str) {
        log::warn!("{}", text);
        self.n_errors += 1;
    }
    pub fn info(&mut self, text: &str) {
        log::info!("{}", text);
    }
    pub fn debug(&mut self, text: &str) {
        log::debug!("{}", text);
    }
    pub fn trace(&mut self, text: &str) {
        log::trace!("{}", text);
    }
}

/// A data source that combines two `CalDavSource`s, which is able to sync both sources.
///
/// Usually, you will only need to use a provider between a server and a local cache, that is to say `Provider<Cache, CachedCalendar, Client, RemoteCalendar>`
/// However, providers can be used for integration tests, where the remote source is mocked by a `Cache`.
pub struct Provider<L, T, R, U>
where
    L: CalDavSource<T>,
    T: CompleteCalendar + Sync + Send,
    R: CalDavSource<U>,
    U: DavCalendar + Sync + Send,
{
    /// The remote source (usually a server)
    remote: R,
    /// The local cache
    local: L,

    phantom_t: PhantomData<T>,
    phantom_u: PhantomData<U>,
}

impl<L, T, R, U> Provider<L, T, R, U>
where
    L: CalDavSource<T>,
    T: CompleteCalendar + Sync + Send,
    R: CalDavSource<U>,
    U: DavCalendar + Sync + Send,
{
    /// Create a provider.
    ///
    /// `remote` is usually a [`Client`](crate::client::Client), `local` is usually a [`Cache`](crate::cache::Cache).
    /// However, both can be interchangeable. The only difference is that `remote` always wins in case of a sync conflict
    pub fn new(remote: R, local: L) -> Self {
        Self { remote, local,
            phantom_t: PhantomData, phantom_u: PhantomData,
        }
    }

    /// Returns the data source described as the `remote`
    pub fn remote(&self) -> &R { &self.remote }
    /// Returns the data source described as the `local`
    pub fn local(&self)  -> &L { &self.local }

    /// Performs a synchronisation between `local` and `remote`.
    ///
    /// This bidirectional sync applies additions/deletions made on a source to the other source.
    /// In case of conflicts (the same item has been modified on both ends since the last sync, `remote` always wins)
    ///
    /// It returns whether the sync was totally successful (details about errors are logged using the `log::*` macros).
    /// In case errors happened, the sync might have been partially executed, and you can safely run this function again, since it has been designed to gracefully recover from errors.
    pub async fn sync(&mut self) -> bool {
        let mut result = SyncResult::new();
        if let Err(err) = self.run_sync(&mut result).await {
            result.error(&format!("Sync terminated because of an error: {}", err));
        }
        result.is_success()
    }

    async fn run_sync(&mut self, result: &mut SyncResult) -> Result<(), Box<dyn Error>> {
        result.info("Starting a sync.");

        let mut handled_calendars = HashSet::new();

        // Sync every remote calendar
        let cals_remote = self.remote.get_calendars().await?;
        for (cal_id, cal_remote) in cals_remote {
            let counterpart = match self.get_or_insert_local_counterpart_calendar(&cal_id, cal_remote.clone()).await {
                Err(err) => {
                    result.warn(&format!("Unable to get or insert local counterpart calendar for {} ({}). Skipping this time", cal_id, err));
                    continue;
                },
                Ok(arc) => arc,
            };

            if let Err(err) = Self::sync_calendar_pair(counterpart, cal_remote, result).await {
                result.warn(&format!("Unable to sync calendar {}: {}, skipping this time.", cal_id, err));
                continue;
            }
            handled_calendars.insert(cal_id);
        }

        // Sync every local calendar that would not be in the remote yet
        let cals_local = self.local.get_calendars().await?;
        for (cal_id, cal_local) in cals_local {
            if handled_calendars.contains(&cal_id) {
                continue;
            }

            let counterpart = match self.get_or_insert_remote_counterpart_calendar(&cal_id, cal_local.clone()).await {
                Err(err) => {
                    result.warn(&format!("Unable to get or insert remote counterpart calendar for {} ({}). Skipping this time", cal_id, err));
                    continue;
                },
                Ok(arc) => arc,
            };

            if let Err(err) = Self::sync_calendar_pair(cal_local, counterpart, result).await {
                result.warn(&format!("Unable to sync calendar {}: {}, skipping this time.", cal_id, err));
                continue;
            }
        }

        Ok(())
    }


    async fn get_or_insert_local_counterpart_calendar(&mut self, cal_id: &CalendarId, needle: Arc<Mutex<U>>) -> Result<Arc<Mutex<T>>, Box<dyn Error>> {
        get_or_insert_counterpart_calendar("local", &mut self.local, cal_id, needle).await
    }
    async fn get_or_insert_remote_counterpart_calendar(&mut self, cal_id: &CalendarId, needle: Arc<Mutex<T>>) -> Result<Arc<Mutex<U>>, Box<dyn Error>> {
        get_or_insert_counterpart_calendar("remote", &mut self.remote, cal_id, needle).await
    }


    async fn sync_calendar_pair(cal_local: Arc<Mutex<T>>, cal_remote: Arc<Mutex<U>>, result: &mut SyncResult) -> Result<(), Box<dyn Error>> {
        let mut cal_remote = cal_remote.lock().unwrap();
        let mut cal_local = cal_local.lock().unwrap();

        // Step 1 - find the differences
        result.debug("Finding the differences to sync...");
        let mut local_del = HashSet::new();
        let mut remote_del = HashSet::new();
        let mut local_changes = HashSet::new();
        let mut remote_changes = HashSet::new();
        let mut local_additions = HashSet::new();
        let mut remote_additions = HashSet::new();

        let remote_items = cal_remote.get_item_version_tags().await?;
        let mut local_items_to_handle = cal_local.get_item_ids().await?;
        for (id, remote_tag) in remote_items {
            result.trace(&format!("***** Considering remote item {}...", id));
            match cal_local.get_item_by_id_ref(&id).await {
                None => {
                    // This was created on the remote
                    result.debug(&format!("*   {} is a remote addition", id));
                    remote_additions.insert(id);
                },
                Some(local_item) => {
                    if local_items_to_handle.remove(&id) == false {
                        result.error(&format!("Inconsistent state: missing task {} from the local tasks", id));
                    }

                    match local_item.sync_status() {
                        SyncStatus::NotSynced => {
                            result.error(&format!("ID reuse between remote and local sources ({}). Ignoring this item in the sync", id));
                            continue;
                        },
                        SyncStatus::Synced(local_tag) => {
                            if &remote_tag != local_tag {
                                // This has been modified on the remote
                                result.debug(&format!("*   {} is a remote change", id));
                                remote_changes.insert(id);
                            }
                        },
                        SyncStatus::LocallyModified(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been changed locally
                                result.debug(&format!("*   {} is a local change", id));
                                local_changes.insert(id);
                            } else {
                                result.info(&format!("Conflict: task {} has been modified in both sources. Using the remote version.", id));
                                result.debug(&format!("*   {} is considered a remote change", id));
                                remote_changes.insert(id);
                            }
                        },
                        SyncStatus::LocallyDeleted(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been locally deleted
                                result.debug(&format!("*   {} is a local deletion", id));
                                local_del.insert(id);
                            } else {
                                result.info(&format!("Conflict: task {} has been locally deleted and remotely modified. Reverting to the remote version.", id));
                                result.debug(&format!("*   {} is a considered a remote change", id));
                                remote_changes.insert(id);
                            }
                        },
                    }
                }
            }
        }

        // Also iterate on the local tasks that are not on the remote
        for id in local_items_to_handle {
            result.trace(&format!("##### Considering local item {}...", id));
            let local_item = match cal_local.get_item_by_id_ref(&id).await {
                None => {
                    result.error(&format!("Inconsistent state: missing task {} from the local tasks", id));
                    continue;
                },
                Some(item) => item,
            };

            match local_item.sync_status() {
                SyncStatus::Synced(_) => {
                    // This item has been removed from the remote
                    result.debug(&format!("#   {} is a deletion from the server", id));
                    remote_del.insert(id);
                },
                SyncStatus::NotSynced => {
                    // This item has just been locally created
                    result.debug(&format!("#   {} has been locally created", id));
                    local_additions.insert(id);
                },
                SyncStatus::LocallyDeleted(_) => {
                    // This item has been deleted from both sources
                    result.debug(&format!("#   {} has been deleted from both sources", id));
                    remote_del.insert(id);
                },
                SyncStatus::LocallyModified(_) => {
                    result.info(&format!("Conflict: item {} has been deleted from the server and locally modified. Deleting the local copy", id));
                    remote_del.insert(id);
                },
            }
        }


        // Step 2 - commit changes
        result.trace("Committing changes...");
        for id_del in local_del {
            result.debug(&format!("> Pushing local deletion {} to the server", id_del));
            match cal_remote.delete_item(&id_del).await {
                Err(err) => {
                    result.warn(&format!("Unable to delete remote item {}: {}", id_del, err));
                },
                Ok(()) => {
                    // Change the local copy from "marked to deletion" to "actually deleted"
                    if let Err(err) = cal_local.immediately_delete_item(&id_del).await {
                        result.error(&format!("Unable to permanently delete local item {}: {}", id_del, err));
                    }
                },
            }
        }

        for id_del in remote_del {
            result.debug(&format!("> Applying remote deletion {} locally", id_del));
            if let Err(err) = cal_local.immediately_delete_item(&id_del).await {
                result.warn(&format!("Unable to delete local item {}: {}", id_del, err));
            }
        }

        for id_add in remote_additions {
            result.debug(&format!("> Applying remote addition {} locally", id_add));
            match cal_remote.get_item_by_id(&id_add).await {
                Err(err) => {
                    result.warn(&format!("Unable to get remote item {}: {}. Skipping it.", id_add, err));
                    continue;
                },
                Ok(item) => match item {
                    None => {
                        result.error(&format!("Inconsistency: new item {} has vanished from the remote end", id_add));
                        continue;
                    },
                    Some(new_item) => {
                        if let Err(err) = cal_local.add_item(new_item.clone()).await {
                            result.error(&format!("Not able to add item {} to local calendar: {}", id_add, err));
                        }
                    },
                },
            }
        }

        for id_change in remote_changes {
            result.debug(&format!("> Applying remote change {} locally", id_change));
            match cal_remote.get_item_by_id(&id_change).await {
                Err(err) => {
                    result.warn(&format!("Unable to get remote item {}: {}. Skipping it", id_change, err));
                    continue;
                },
                Ok(item) => match item {
                    None => {
                        result.error(&format!("Inconsistency: modified item {} has vanished from the remote end", id_change));
                        continue;
                    },
                    Some(item) => {
                        if let Err(err) = cal_local.update_item(item.clone()).await {
                            result.error(&format!("Unable to update item {} in local calendar: {}", id_change, err));
                        }
                    },
                }
            }
        }


        for id_add in local_additions {
            result.debug(&format!("> Pushing local addition {} to the server", id_add));
            match cal_local.get_item_by_id_mut(&id_add).await {
                None => {
                    result.error(&format!("Inconsistency: created item {} has been marked for upload but is locally missing", id_add));
                    continue;
                },
                Some(item) => {
                    match cal_remote.add_item(item.clone()).await {
                        Err(err) => result.error(&format!("Unable to add item {} to remote calendar: {}", id_add, err)),
                        Ok(new_ss) => {
                            // Update local sync status
                            item.set_sync_status(new_ss);
                        },
                    }
                },
            };
        }

        for id_change in local_changes {
            result.debug(&format!("> Pushing local change {} to the server", id_change));
            match cal_local.get_item_by_id_mut(&id_change).await {
                None => {
                    result.error(&format!("Inconsistency: modified item {} has been marked for upload but is locally missing", id_change));
                    continue;
                },
                Some(item) => {
                    match cal_remote.update_item(item.clone()).await {
                        Err(err) => result.error(&format!("Unable to update item {} in remote calendar: {}", id_change, err)),
                        Ok(new_ss) => {
                            // Update local sync status
                            item.set_sync_status(new_ss);
                        },
                    };
                }
            };
        }

        Ok(())
    }
}


pub async fn get_or_insert_counterpart_calendar<H, N, I>(haystack_descr: &str, haystack: &mut H, cal_id: &CalendarId, needle: Arc<Mutex<N>>)
    -> Result<Arc<Mutex<I>>, Box<dyn Error>>
where
    H: CalDavSource<I>,
    I: BaseCalendar,
    N: BaseCalendar,
{
    loop {
        if let Some(cal) = haystack.get_calendar(&cal_id).await {
            break Ok(cal);
        }

        // This calendar does not exist locally yet, let's add it
        log::debug!("Adding a {} calendar {}", haystack_descr, cal_id);
        let src = needle.lock().unwrap();
        let name = src.name().to_string();
        let supported_comps = src.supported_components();
        if let Err(err) = haystack.create_calendar(
            cal_id.clone(),
            name,
            supported_comps,
        ).await{
            return Err(err);
        }
    }
}

