//! This modules abstracts data sources and merges them in a single virtual one
//!
//! It is also responsible for syncing them together

use std::error::Error;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::traits::{BaseCalendar, CalDavSource, DavCalendar};
use crate::traits::CompleteCalendar;
use crate::item::{ItemId, SyncStatus};
use crate::calendar::CalendarId;

pub mod sync_progress;
use sync_progress::SyncProgress;
use sync_progress::{FeedbackSender, SyncEvent};

/// A data source that combines two `CalDavSource`s, which is able to sync both sources.
///
/// Usually, you will only need to use a provider between a server and a local cache, that is to say a [`CalDavProvider`](crate::CalDavProvider), i.e. a `Provider<Cache, CachedCalendar, Client, RemoteCalendar>`. \
/// However, providers can be used for integration tests, where the remote source is mocked by a `Cache`.
#[derive(Debug)]
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

    /// Returns the data source described as `local`
    pub fn local(&self)  -> &L { &self.local }
    /// Returns the data source described as `local`
    pub fn local_mut(&mut self)  -> &mut L { &mut self.local }
    /// Returns the data source described as `remote`.
    ///
    /// Apart from tests, there are very few (if any) reasons to access `remote` directly.
    /// Usually, you should rather use the `local` source, which (usually) is a much faster local cache.
    /// To be sure `local` accurately mirrors the `remote` source, you can run [`Provider::sync`]
    pub fn remote(&self) -> &R { &self.remote }

    /// Performs a synchronisation between `local` and `remote`, and provide feeedback to the user about the progress.
    ///
    /// This bidirectional sync applies additions/deletions made on a source to the other source.
    /// In case of conflicts (the same item has been modified on both ends since the last sync, `remote` always wins)
    ///
    /// It returns whether the sync was totally successful (details about errors are logged using the `log::*` macros).
    /// In case errors happened, the sync might have been partially executed, and you can safely run this function again, since it has been designed to gracefully recover from errors.
    pub async fn sync_with_feedback(&mut self, feedback_sender: FeedbackSender) -> bool {
        let mut progress = SyncProgress::new_with_feedback_channel(feedback_sender);
        self.run_sync(&mut progress).await
    }

    /// Performs a synchronisation between `local` and `remote`, without giving any feedback.
    ///
    /// See [sync_with_feedback]
    pub async fn sync(&mut self) -> bool {
        let mut progress = SyncProgress::new();
        self.run_sync(&mut progress).await
    }

    async fn run_sync(&mut self, progress: &mut SyncProgress) -> bool {
        if let Err(err) = self.run_sync_inner(progress).await {
            progress.error(&format!("Sync terminated because of an error: {}", err));
        }
        progress.feedback(SyncEvent::Finished{ success: progress.is_success() });
        progress.is_success()
    }

    async fn run_sync_inner(&mut self, progress: &mut SyncProgress) -> Result<(), Box<dyn Error>> {
        progress.info("Starting a sync.");
        progress.feedback(SyncEvent::Started);

        let mut handled_calendars = HashSet::new();

        // Sync every remote calendar
        let cals_remote = self.remote.get_calendars().await?;
        for (cal_id, cal_remote) in cals_remote {
            let counterpart = match self.get_or_insert_local_counterpart_calendar(&cal_id, cal_remote.clone()).await {
                Err(err) => {
                    progress.warn(&format!("Unable to get or insert local counterpart calendar for {} ({}). Skipping this time", cal_id, err));
                    continue;
                },
                Ok(arc) => arc,
            };

            if let Err(err) = Self::sync_calendar_pair(counterpart, cal_remote, progress).await {
                progress.warn(&format!("Unable to sync calendar {}: {}, skipping this time.", cal_id, err));
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
                    progress.warn(&format!("Unable to get or insert remote counterpart calendar for {} ({}). Skipping this time", cal_id, err));
                    continue;
                },
                Ok(arc) => arc,
            };

            if let Err(err) = Self::sync_calendar_pair(cal_local, counterpart, progress).await {
                progress.warn(&format!("Unable to sync calendar {}: {}, skipping this time.", cal_id, err));
                continue;
            }
        }

        progress.info("Sync ended");

        Ok(())
    }


    async fn get_or_insert_local_counterpart_calendar(&mut self, cal_id: &CalendarId, needle: Arc<Mutex<U>>) -> Result<Arc<Mutex<T>>, Box<dyn Error>> {
        get_or_insert_counterpart_calendar("local", &mut self.local, cal_id, needle).await
    }
    async fn get_or_insert_remote_counterpart_calendar(&mut self, cal_id: &CalendarId, needle: Arc<Mutex<T>>) -> Result<Arc<Mutex<U>>, Box<dyn Error>> {
        get_or_insert_counterpart_calendar("remote", &mut self.remote, cal_id, needle).await
    }


    async fn sync_calendar_pair(cal_local: Arc<Mutex<T>>, cal_remote: Arc<Mutex<U>>, progress: &mut SyncProgress) -> Result<(), Box<dyn Error>> {
        let mut cal_remote = cal_remote.lock().unwrap();
        let mut cal_local = cal_local.lock().unwrap();
        let cal_name = cal_local.name().to_string();

        progress.info(&format!("Syncing calendar {}", cal_name));
        progress.feedback(SyncEvent::InProgress{
            calendar: cal_name.clone(),
            details: "started".to_string()
        });

        // Step 1 - find the differences
        progress.debug("Finding the differences to sync...");
        let mut local_del = HashSet::new();
        let mut remote_del = HashSet::new();
        let mut local_changes = HashSet::new();
        let mut remote_changes = HashSet::new();
        let mut local_additions = HashSet::new();
        let mut remote_additions = HashSet::new();

        let remote_items = cal_remote.get_item_version_tags().await?;
        progress.feedback(SyncEvent::InProgress{
            calendar: cal_name.clone(),
            details: format!("{} remote items", remote_items.len()),
        });

        let mut local_items_to_handle = cal_local.get_item_ids().await?;
        for (id, remote_tag) in remote_items {
            progress.trace(&format!("***** Considering remote item {}...", id));
            match cal_local.get_item_by_id(&id).await {
                None => {
                    // This was created on the remote
                    progress.debug(&format!("*   {} is a remote addition", id));
                    remote_additions.insert(id);
                },
                Some(local_item) => {
                    if local_items_to_handle.remove(&id) == false {
                        progress.error(&format!("Inconsistent state: missing task {} from the local tasks", id));
                    }

                    match local_item.sync_status() {
                        SyncStatus::NotSynced => {
                            progress.error(&format!("ID reuse between remote and local sources ({}). Ignoring this item in the sync", id));
                            continue;
                        },
                        SyncStatus::Synced(local_tag) => {
                            if &remote_tag != local_tag {
                                // This has been modified on the remote
                                progress.debug(&format!("*   {} is a remote change", id));
                                remote_changes.insert(id);
                            }
                        },
                        SyncStatus::LocallyModified(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been changed locally
                                progress.debug(&format!("*   {} is a local change", id));
                                local_changes.insert(id);
                            } else {
                                progress.info(&format!("Conflict: task {} has been modified in both sources. Using the remote version.", id));
                                progress.debug(&format!("*   {} is considered a remote change", id));
                                remote_changes.insert(id);
                            }
                        },
                        SyncStatus::LocallyDeleted(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been locally deleted
                                progress.debug(&format!("*   {} is a local deletion", id));
                                local_del.insert(id);
                            } else {
                                progress.info(&format!("Conflict: task {} has been locally deleted and remotely modified. Reverting to the remote version.", id));
                                progress.debug(&format!("*   {} is a considered a remote change", id));
                                remote_changes.insert(id);
                            }
                        },
                    }
                }
            }
        }

        // Also iterate on the local tasks that are not on the remote
        for id in local_items_to_handle {
            progress.trace(&format!("##### Considering local item {}...", id));
            let local_item = match cal_local.get_item_by_id(&id).await {
                None => {
                    progress.error(&format!("Inconsistent state: missing task {} from the local tasks", id));
                    continue;
                },
                Some(item) => item,
            };

            match local_item.sync_status() {
                SyncStatus::Synced(_) => {
                    // This item has been removed from the remote
                    progress.debug(&format!("#   {} is a deletion from the server", id));
                    remote_del.insert(id);
                },
                SyncStatus::NotSynced => {
                    // This item has just been locally created
                    progress.debug(&format!("#   {} has been locally created", id));
                    local_additions.insert(id);
                },
                SyncStatus::LocallyDeleted(_) => {
                    // This item has been deleted from both sources
                    progress.debug(&format!("#   {} has been deleted from both sources", id));
                    remote_del.insert(id);
                },
                SyncStatus::LocallyModified(_) => {
                    progress.info(&format!("Conflict: item {} has been deleted from the server and locally modified. Deleting the local copy", id));
                    remote_del.insert(id);
                },
            }
        }


        // Step 2 - commit changes
        progress.trace("Committing changes...");
        for id_del in local_del {
            progress.debug(&format!("> Pushing local deletion {} to the server", id_del));
            progress.feedback(SyncEvent::InProgress{
                calendar: cal_name.clone(),
                details: Self::item_name(&cal_local, &id_del).await,
            });
            match cal_remote.delete_item(&id_del).await {
                Err(err) => {
                    progress.warn(&format!("Unable to delete remote item {}: {}", id_del, err));
                },
                Ok(()) => {
                    // Change the local copy from "marked to deletion" to "actually deleted"
                    if let Err(err) = cal_local.immediately_delete_item(&id_del).await {
                        progress.error(&format!("Unable to permanently delete local item {}: {}", id_del, err));
                    }
                },
            }
        }

        for id_del in remote_del {
            progress.debug(&format!("> Applying remote deletion {} locally", id_del));
            progress.feedback(SyncEvent::InProgress{
                calendar: cal_name.clone(),
                details: Self::item_name(&cal_local, &id_del).await,
            });
            if let Err(err) = cal_local.immediately_delete_item(&id_del).await {
                progress.warn(&format!("Unable to delete local item {}: {}", id_del, err));
            }
        }

        for id_add in remote_additions {
            progress.debug(&format!("> Applying remote addition {} locally", id_add));
            progress.feedback(SyncEvent::InProgress{
                calendar: cal_name.clone(),
                details: Self::item_name(&cal_local, &id_add).await,
            });
            match cal_remote.get_item_by_id(&id_add).await {
                Err(err) => {
                    progress.warn(&format!("Unable to get remote item {}: {}. Skipping it.", id_add, err));
                    continue;
                },
                Ok(item) => match item {
                    None => {
                        progress.error(&format!("Inconsistency: new item {} has vanished from the remote end", id_add));
                        continue;
                    },
                    Some(new_item) => {
                        if let Err(err) = cal_local.add_item(new_item.clone()).await {
                            progress.error(&format!("Not able to add item {} to local calendar: {}", id_add, err));
                        }
                    },
                },
            }
        }

        for id_change in remote_changes {
            progress.debug(&format!("> Applying remote change {} locally", id_change));
            progress.feedback(SyncEvent::InProgress{
                calendar: cal_name.clone(),
                details: Self::item_name(&cal_local, &id_change).await,
            });
            match cal_remote.get_item_by_id(&id_change).await {
                Err(err) => {
                    progress.warn(&format!("Unable to get remote item {}: {}. Skipping it", id_change, err));
                    continue;
                },
                Ok(item) => match item {
                    None => {
                        progress.error(&format!("Inconsistency: modified item {} has vanished from the remote end", id_change));
                        continue;
                    },
                    Some(item) => {
                        if let Err(err) = cal_local.update_item(item.clone()).await {
                            progress.error(&format!("Unable to update item {} in local calendar: {}", id_change, err));
                        }
                    },
                }
            }
        }


        for id_add in local_additions {
            progress.debug(&format!("> Pushing local addition {} to the server", id_add));
            progress.feedback(SyncEvent::InProgress{
                calendar: cal_name.clone(),
                details: Self::item_name(&cal_local, &id_add).await,
            });
            match cal_local.get_item_by_id_mut(&id_add).await {
                None => {
                    progress.error(&format!("Inconsistency: created item {} has been marked for upload but is locally missing", id_add));
                    continue;
                },
                Some(item) => {
                    match cal_remote.add_item(item.clone()).await {
                        Err(err) => progress.error(&format!("Unable to add item {} to remote calendar: {}", id_add, err)),
                        Ok(new_ss) => {
                            // Update local sync status
                            item.set_sync_status(new_ss);
                        },
                    }
                },
            };
        }

        for id_change in local_changes {
            progress.debug(&format!("> Pushing local change {} to the server", id_change));
            progress.feedback(SyncEvent::InProgress{
                calendar: cal_name.clone(),
                details: Self::item_name(&cal_local, &id_change).await,
            });
            match cal_local.get_item_by_id_mut(&id_change).await {
                None => {
                    progress.error(&format!("Inconsistency: modified item {} has been marked for upload but is locally missing", id_change));
                    continue;
                },
                Some(item) => {
                    match cal_remote.update_item(item.clone()).await {
                        Err(err) => progress.error(&format!("Unable to update item {} in remote calendar: {}", id_change, err)),
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


    async fn item_name(cal: &T, id: &ItemId) -> String {
        cal.get_item_by_id(id).await.map(|item| item.name()).unwrap_or_default().to_string()
    }

}


async fn get_or_insert_counterpart_calendar<H, N, I>(haystack_descr: &str, haystack: &mut H, cal_id: &CalendarId, needle: Arc<Mutex<N>>)
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
        let color = src.color();
        if let Err(err) = haystack.create_calendar(
            cal_id.clone(),
            name,
            supported_comps,
            color.cloned(),
        ).await{
            return Err(err);
        }
    }
}

