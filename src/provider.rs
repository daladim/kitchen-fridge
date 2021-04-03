//! This modules abstracts data sources and merges them in a single virtual one

use std::error::Error;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::traits::{CalDavSource, DavCalendar};
use crate::traits::CompleteCalendar;
use crate::item::SyncStatus;
use crate::calendar::SupportedComponents;
use crate::calendar::CalendarId;

/// A data source that combines two `CalDavSource`s (usually a server and a local cache), which is able to sync both sources.
/// This can be used for integration tests, where the remote source is mocked by a `Cache`.
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
    pub async fn sync(&mut self) -> Result<(), Box<dyn Error>> {
        log::info!("Starting a sync.");

        let mut handled_calendars = HashSet::new();

        // Sync every remote calendar
        let cals_remote = self.remote.get_calendars().await?;
        for (cal_id, cal_remote) in cals_remote {
            let cal_local = self.get_or_insert_local_counterpart_calendar(&cal_id).await;

            if let Err(err) = Self::sync_calendar_pair(cal_local, cal_remote).await {
                log::warn!("Unable to sync calendar {}: {}, skipping this time.", cal_id, err);
            }
            handled_calendars.insert(cal_id);
        }

        // Sync every local calendar that would not be in the remote yet
        let cals_local = self.local.get_calendars().await?;
        for (cal_id, cal_local) in cals_local {
            if handled_calendars.contains(&cal_id) {
                continue;
            }

            let cal_remote = self.get_or_insert_remote_counterpart_calendar(&cal_id).await;

            if let Err(err) = Self::sync_calendar_pair(cal_local, cal_remote).await {
                log::warn!("Unable to sync calendar {}: {}, skipping this time.", cal_id, err);
            }
        }

        Ok(())
    }


    async fn get_or_insert_local_counterpart_calendar(&mut self, cal_id: &CalendarId) -> Arc<Mutex<T>> {
        loop {
            if let Some(cal) = self.local.get_calendar(&cal_id).await {
                break cal;
            }

            // This calendar does not exist locally yet, let's add it
            log::debug!("Adding a local calendar {}", cal_id);
            if let Err(err) = self.local.create_calendar(
                cal_id.clone(),
                String::from("new calendar"),
                SupportedComponents::TODO,
            ).await {
                log::warn!("Unable to create local calendar {}: {}. Skipping it.", cal_id, err);
                continue;
            }
        }
    }

    async fn get_or_insert_remote_counterpart_calendar(&mut self, cal_id: &CalendarId) -> Arc<Mutex<U>> {
        loop {
            if let Some(cal) = self.remote.get_calendar(&cal_id).await {
                break cal;
            }

            // This calendar does not exist in the remote yet, let's add it
            log::debug!("Adding a remote calendar {}", cal_id);
            if let Err(err) = self.remote.create_calendar(
                cal_id.clone(),
                String::from("new calendar"),
                SupportedComponents::TODO,
            ).await {
                log::warn!("Unable to create remote calendar {}: {}. Skipping it.", cal_id, err);
                continue;
            }
        }
    }



    async fn sync_calendar_pair(cal_local: Arc<Mutex<T>>, cal_remote: Arc<Mutex<U>>) -> Result<(), Box<dyn Error>> {
        let mut cal_remote = cal_remote.lock().unwrap();
        let mut cal_local = cal_local.lock().unwrap();

        // Step 1 - find the differences
        log::debug!("Finding the differences to sync...");
        let mut local_del = HashSet::new();
        let mut remote_del = HashSet::new();
        let mut local_changes = HashSet::new();
        let mut remote_changes = HashSet::new();
        let mut local_additions = HashSet::new();
        let mut remote_additions = HashSet::new();

        let remote_items = cal_remote.get_item_version_tags().await?;
        let mut local_items_to_handle = cal_local.get_item_ids().await?;
        for (id, remote_tag) in remote_items {
            log::trace!("***** Considering remote item {}...", id);
            match cal_local.get_item_by_id_ref(&id).await {
                None => {
                    // This was created on the remote
                    log::debug!("*   {} is a remote addition", id);
                    remote_additions.insert(id);
                },
                Some(local_item) => {
                    if local_items_to_handle.remove(&id) == false {
                        log::error!("Inconsistent state: missing task {} from the local tasks", id);
                    }

                    match local_item.sync_status() {
                        SyncStatus::NotSynced => {
                            log::error!("ID reuse between remote and local sources ({}). Ignoring this item in the sync", id);
                            continue;
                        },
                        SyncStatus::Synced(local_tag) => {
                            if &remote_tag != local_tag {
                                // This has been modified on the remote
                                log::debug!("*   {} is a remote change", id);
                                remote_changes.insert(id);
                            }
                        },
                        SyncStatus::LocallyModified(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been changed locally
                                log::debug!("*   {} is a local change", id);
                                local_changes.insert(id);
                            } else {
                                log::info!("Conflict: task {} has been modified in both sources. Using the remote version.", id);
                                log::debug!("*   {} is considered a remote change", id);
                                remote_changes.insert(id);
                            }
                        },
                        SyncStatus::LocallyDeleted(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been locally deleted
                                log::debug!("*   {} is a local deletion", id);
                                local_del.insert(id);
                            } else {
                                log::info!("Conflict: task {} has been locally deleted and remotely modified. Reverting to the remote version.", id);
                                log::debug!("*   {} is a considered a remote change", id);
                                remote_changes.insert(id);
                            }
                        },
                    }
                }
            }
        }

        // Also iterate on the local tasks that are not on the remote
        for id in local_items_to_handle {
            log::trace!("##### Considering local item {}...", id);
            let local_item = match cal_local.get_item_by_id_ref(&id).await {
                None => {
                    log::error!("Inconsistent state: missing task {} from the local tasks", id);
                    continue;
                },
                Some(item) => item,
            };

            match local_item.sync_status() {
                SyncStatus::Synced(_) => {
                    // This item has been removed from the remote
                    log::debug!("#   {} is a deletion from the server", id);
                    remote_del.insert(id);
                },
                SyncStatus::NotSynced => {
                    // This item has just been locally created
                    log::debug!("#   {} has been locally created", id);
                    local_additions.insert(id);
                },
                SyncStatus::LocallyDeleted(_) => {
                    // This item has been deleted from both sources
                    log::debug!("#   {} has been deleted from both sources", id);
                    remote_del.insert(id);
                },
                SyncStatus::LocallyModified(_) => {
                    log::info!("Conflict: item {} has been deleted from the server and locally modified. Deleting the local copy", id);
                    remote_del.insert(id);
                },
            }
        }


        // Step 2 - commit changes
        log::trace!("Committing changes...");
        for id_del in local_del {
            log::debug!("> Pushing local deletion {} to the server", id_del);
            match cal_remote.delete_item(&id_del).await {
                Err(err) => {
                    log::warn!("Unable to delete remote item {}: {}", id_del, err);
                },
                Ok(()) => {
                    // Change the local copy from "marked to deletion" to "actually deleted"
                    if let Err(err) = cal_local.immediately_delete_item(&id_del).await {
                        log::error!("Unable to permanently delete local item {}: {}", id_del, err);
                    }
                },
            }
        }

        for id_del in remote_del {
            log::debug!("> Applying remote deletion {} locally", id_del);
            if let Err(err) = cal_local.immediately_delete_item(&id_del).await {
                log::warn!("Unable to delete local item {}: {}", id_del, err);
            }
        }

        for id_add in remote_additions {
            log::debug!("> Applying remote addition {} locally", id_add);
            match cal_remote.get_item_by_id(&id_add).await {
                Err(err) => {
                    log::warn!("Unable to get remote item {}: {}. Skipping it.", id_add, err);
                    continue;
                },
                Ok(item) => match item {
                    None => {
                        log::error!("Inconsistency: new item {} has vanished from the remote end", id_add);
                        continue;
                    },
                    Some(new_item) => {
                        if let Err(err) = cal_local.add_item(new_item.clone()).await {
                            log::error!("Not able to add item {} to local calendar: {}", id_add, err);
                        }
                    },
                },
            }
        }

        for id_change in remote_changes {
            log::debug!("> Applying remote change {} locally", id_change);
            match cal_remote.get_item_by_id(&id_change).await {
                Err(err) => {
                    log::warn!("Unable to get remote item {}: {}. Skipping it", id_change, err);
                    continue;
                },
                Ok(item) => match item {
                    None => {
                        log::error!("Inconsistency: modified item {} has vanished from the remote end", id_change);
                        continue;
                    },
                    Some(item) => {
                        //
                        //
                        //
                        //
                        // TODO: implement update_item (maybe only create_item also updates it?)
                        //
                        if let Err(err) = cal_local.immediately_delete_item(&id_change).await {
                            log::error!("Unable to delete item {} from local calendar: {}", id_change, err);
                        }
                        if let Err(err) = cal_local.add_item(item.clone()).await {
                            log::error!("Unable to add item {} to local calendar: {}", id_change, err);
                        }
                    },
                }
            }
        }


        for id_add in local_additions {
            log::debug!("> Pushing local addition {} to the server", id_add);
            match cal_local.get_item_by_id_mut(&id_add).await {
                None => {
                    log::error!("Inconsistency: created item {} has been marked for upload but is locally missing", id_add);
                    continue;
                },
                Some(item) => {
                    match cal_remote.add_item(item.clone()).await {
                        Err(err) => log::error!("Unable to add item {} to remote calendar: {}", id_add, err),
                        Ok(new_ss) => {
                            // Update local sync status
                            item.set_sync_status(new_ss);
                        },
                    }
                },
            };
        }

        for id_change in local_changes {
            log::debug!("> Pushing local change {} to the server", id_change);
            match cal_local.get_item_by_id_mut(&id_change).await {
                None => {
                    log::error!("Inconsistency: modified item {} has been marked for upload but is locally missing", id_change);
                    continue;
                },
                Some(item) => {
                    //
                    //
                    //
                    //
                    // TODO: implement update_item (maybe only create_item also updates it?)
                    //
                    if let Err(err) = cal_remote.delete_item(&id_change).await {
                        log::error!("Unable to delete item {} from remote calendar: {}", id_change, err);
                    }
                    match cal_remote.add_item(item.clone()).await {
                        Err(err) => log::error!("Unable to add item {} to remote calendar: {}", id_change, err),
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

