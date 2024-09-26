//! This modules abstracts data sources and merges them in a single virtual one
//!
//! It is also responsible for syncing them together

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter, Write};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use itertools::Itertools;
use url::Url;

use crate::error::KFResult;
use crate::traits::CompleteCalendar;
use crate::traits::{BaseCalendar, CalDavSource, DavCalendar};
use crate::utils::prop::Property;
use crate::utils::sync::{SyncStatus, Syncable};
use crate::utils::NamespacedName;

pub mod sync_progress;
use sync_progress::SyncProgress;
use sync_progress::{FeedbackSender, SyncEvent};

/// How many items will be batched in a single HTTP request when downloading from the server
#[cfg(not(test))]
const DOWNLOAD_BATCH_SIZE: usize = 30;
/// How many items will be batched in a single HTTP request when downloading from the server
#[cfg(test)]
const DOWNLOAD_BATCH_SIZE: usize = 3;

// I am too lazy to actually make `fetch_and_apply` generic over an async closure.
// Let's work around by passing an enum, so that `fetch_and_apply` will know what to do
enum BatchDownloadType {
    RemoteAdditions,
    RemoteChanges,
}

impl Display for BatchDownloadType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::RemoteAdditions => write!(f, "remote additions"),
            Self::RemoteChanges => write!(f, "remote changes"),
        }
    }
}

struct ItemChanges {
    local_item_dels: HashSet<Url>,
    remote_item_dels: HashSet<Url>,
    local_item_changes: HashSet<Url>,
    remote_item_changes: HashSet<Url>,
    local_item_additions: HashSet<Url>,
    remote_item_additions: HashSet<Url>,
}

struct PropChanges {
    local_prop_dels: HashSet<NamespacedName>,
    remote_prop_dels: HashSet<NamespacedName>,
    local_prop_changes: HashSet<NamespacedName>,
    remote_prop_changes: HashSet<Property>,
    local_prop_additions: HashSet<Property>,
    remote_prop_additions: HashSet<Property>,
}
impl std::fmt::Debug for PropChanges {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("local_prop_dels:")?;
        for x in &self.local_prop_dels {
            f.write_str(format!("\n* {}", x).as_str())?;
        }
        f.write_str("\nremote_prop_dels:")?;
        for x in &self.remote_prop_dels {
            f.write_str(format!("\n* {}", x).as_str())?;
        }
        f.write_str("\nlocal_prop_changes:")?;
        for x in &self.local_prop_changes {
            f.write_str(format!("\n* {}", x).as_str())?;
        }
        f.write_str("\nremote_prop_changes:")?;
        for x in &self.remote_prop_changes {
            f.write_str(format!("\n* {}", x).as_str())?;
        }
        f.write_str("\nlocal_prop_additions:")?;
        for x in &self.local_prop_additions {
            f.write_str(format!("\n* {}", x).as_str())?;
        }
        f.write_str("\nremote_prop_additions:")?;
        for x in &self.remote_prop_additions {
            f.write_str(format!("\n* {}", x).as_str())?;
        }
        f.write_char('\n')
    }
}

/// A data source that combines two `CalDavSource`s, which is able to sync both sources.
///
/// Usually, you will only need to use a provider between a server and a local cache, that is to say a [`CalDavProvider`](crate::CalDavProvider),
/// i.e. a `Provider<Cache, CachedCalendar, Client, RemoteCalendar>`. However, providers can be used for integration tests, where the remote
/// source is mocked by a `Cache`.
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
        Self {
            remote,
            local,
            phantom_t: PhantomData,
            phantom_u: PhantomData,
        }
    }

    /// Returns the data source described as `local`
    pub fn local(&self) -> &L {
        &self.local
    }
    /// Returns the data source described as `local`
    pub fn local_mut(&mut self) -> &mut L {
        &mut self.local
    }
    /// Returns the data source described as `remote`.
    ///
    /// Apart from tests, there are very few (if any) reasons to access `remote` directly.
    /// Usually, you should rather use the `local` source, which (usually) is a much faster local cache.
    /// To be sure `local` accurately mirrors the `remote` source, you can run [`Provider::sync`]
    pub fn remote(&self) -> &R {
        &self.remote
    }

    /// Performs a synchronisation between `local` and `remote`, and provide feeedback to the user about the progress.
    ///
    /// This bidirectional sync applies additions/deletions made on a source to the other source.
    /// In case of conflicts (the same item has been modified on both ends since the last sync, `remote` always wins).
    ///
    /// It returns whether the sync was totally successful (details about errors are logged using the `log::*` macros).
    /// In case errors happened, the sync might have been partially executed but your data will never be correupted (either locally nor in the server).
    /// Simply run this function again, it will re-start a sync, picking up where it failed.
    pub async fn sync_with_feedback(&mut self, feedback_sender: FeedbackSender) -> bool {
        let mut progress = SyncProgress::new_with_feedback_channel(feedback_sender);
        self.run_sync(&mut progress).await
    }

    /// Performs a synchronisation between `local` and `remote`, without giving any feedback.
    ///
    /// See [`Self::sync_with_feedback`]
    pub async fn sync(&mut self) -> bool {
        let mut progress = SyncProgress::new();
        self.run_sync(&mut progress).await
    }

    async fn run_sync(&mut self, progress: &mut SyncProgress) -> bool {
        if let Err(err) = self.run_sync_inner(progress).await {
            progress.error(&format!("Sync terminated because of an error: {}", err));
        }
        progress.feedback(SyncEvent::Finished {
            success: progress.is_success(),
        });
        progress.is_success()
    }

    async fn run_sync_inner(&mut self, progress: &mut SyncProgress) -> KFResult<()> {
        progress.info("Starting a sync.");
        progress.feedback(SyncEvent::Started);

        let mut handled_calendars = HashSet::new();

        // Sync every remote calendar
        let cals_remote = self.remote.get_calendars().await?;
        for (cal_url, cal_remote) in cals_remote {
            let counterpart = match self
                .get_or_insert_local_counterpart_calendar(&cal_url, cal_remote.clone())
                .await
            {
                Err(err) => {
                    progress.warn(&format!("Unable to get or insert local counterpart calendar for {} ({}). Skipping this time", cal_url, err));
                    continue;
                }
                Ok(arc) => arc,
            };

            if let Err(err) = self
                .sync_calendar_pair(counterpart, cal_remote, progress)
                .await
            {
                progress.warn(&format!(
                    "Unable to sync calendar {}: {}, skipping this time.",
                    cal_url, err
                ));
                continue;
            }
            handled_calendars.insert(cal_url);
        }

        // Sync every local calendar that would not be in the remote yet
        let cals_local = self.local.get_calendars().await?;
        for (cal_url, cal_local) in cals_local {
            if handled_calendars.contains(&cal_url) {
                continue;
            }

            if cal_local.lock().unwrap().marked_for_deletion().await {
                self.local_mut().delete_calendar(&cal_url).await?;
                continue;
            }

            let counterpart = match self
                .get_or_insert_remote_counterpart_calendar(&cal_url, cal_local.clone())
                .await
            {
                Err(err) => {
                    progress.warn(&format!("Unable to get or insert remote counterpart calendar for {} ({}). Skipping this time", cal_url, err));
                    continue;
                }
                Ok(arc) => arc,
            };

            if let Err(err) = self
                .sync_calendar_pair(cal_local, counterpart, progress)
                .await
            {
                progress.warn(&format!(
                    "Unable to sync calendar {}: {}, skipping this time.",
                    cal_url, err
                ));
                continue;
            }
        }

        progress.info("Sync ended");

        Ok(())
    }

    async fn get_or_insert_local_counterpart_calendar(
        &mut self,
        cal_url: &Url,
        needle: Arc<Mutex<U>>,
    ) -> KFResult<Arc<Mutex<T>>> {
        get_or_insert_counterpart_calendar("local", &mut self.local, cal_url, needle).await
    }
    async fn get_or_insert_remote_counterpart_calendar(
        &mut self,
        cal_url: &Url,
        needle: Arc<Mutex<T>>,
    ) -> KFResult<Arc<Mutex<U>>> {
        get_or_insert_counterpart_calendar("remote", &mut self.remote, cal_url, needle).await
    }

    async fn sync_calendar_pair(
        &mut self,
        cal_local: Arc<Mutex<T>>,
        cal_remote: Arc<Mutex<U>>,
        progress: &mut SyncProgress,
    ) -> KFResult<()> {
        let mut cal_remote = cal_remote.lock().unwrap();
        let mut cal_local = cal_local.lock().unwrap();
        let cal_name = cal_local.name().to_string();

        progress.info(&format!("Syncing calendar {}", cal_name));
        progress.reset_counter();
        progress.feedback(SyncEvent::ItemsInProgress {
            calendar_name: cal_name.clone(),
            items_done_already: 0,
            details: "started".to_string(),
        });

        // Step 0 - if the local calendar is marked for deletion, remove it from the remote and the local providers
        if cal_local.marked_for_deletion().await {
            self.remote
                .delete_calendar(cal_local.url())
                .await
                .map(|_| ())?;
            self.local
                .delete_calendar(cal_local.url())
                .await
                .map(|_| ())?;
            return Ok(());
        }

        // Step 1 - find the differences
        progress.debug("Finding the differences to sync...");

        // - Step 1.1 - find the differences in items
        let item_changes =
            Self::calculate_item_changes(&cal_local, &cal_remote, progress, cal_name.clone())
                .await?;

        // - Step 1.2 - find the differences in properties
        let prop_changes =
            Self::calculate_prop_changes(&cal_local, &cal_remote, progress, cal_name.clone())
                .await?;

        log::debug!("Prop changes: {:?}", prop_changes);

        // Step 2 - commit changes to tasks
        Self::commit_item_changes(
            &mut cal_local,
            &mut cal_remote,
            progress,
            cal_name.clone(),
            item_changes,
        )
        .await?;

        // Step 3 - commit changes to props
        Self::commit_prop_changes(
            &mut cal_local,
            &mut cal_remote,
            progress,
            cal_name.clone(),
            prop_changes,
        )
        .await?;

        Ok(())
    }

    /// Summarizes the delta between local and remote
    async fn calculate_item_changes(
        cal_local: &T,
        cal_remote: &U,
        progress: &mut SyncProgress,
        cal_name: String,
    ) -> KFResult<ItemChanges> {
        let mut local_item_dels = HashSet::new();
        let mut remote_item_dels = HashSet::new();
        let mut local_item_changes = HashSet::new();
        let mut remote_item_changes = HashSet::new();
        let mut local_item_additions = HashSet::new();
        let mut remote_item_additions = HashSet::new();

        let remote_items = cal_remote.get_item_version_tags().await?;
        progress.feedback(SyncEvent::ItemsInProgress {
            calendar_name: cal_name.clone(),
            items_done_already: 0,
            details: format!("{} remote items", remote_items.len()),
        });

        let mut local_items_to_handle = cal_local.get_item_urls().await?;
        for (url, remote_tag) in remote_items {
            progress.trace(&format!("***** Considering remote item {}...", url));
            match cal_local.get_item_by_url(&url).await {
                None => {
                    // This was created on the remote
                    progress.debug(&format!("*   {} is a remote addition", url));
                    remote_item_additions.insert(url);
                }
                Some(local_item) => {
                    if !local_items_to_handle.remove(&url) {
                        progress.error(&format!(
                            "Inconsistent state: missing task {} from the local tasks",
                            url
                        ));
                    }

                    match local_item.sync_status() {
                        SyncStatus::NotSynced => {
                            progress.error(&format!("URL reuse between remote and local sources ({}). Ignoring this item in the sync", url));
                            continue;
                        }
                        SyncStatus::Synced(local_tag) => {
                            if &remote_tag != local_tag {
                                // This has been modified on the remote
                                progress.debug(&format!("*   {} is a remote change", url));
                                remote_item_changes.insert(url);
                            }
                        }
                        SyncStatus::LocallyModified(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been changed locally
                                progress.debug(&format!("*   {} is a local change", url));
                                local_item_changes.insert(url);
                            } else {
                                progress.info(&format!("Conflict: task {} has been modified in both sources. Using the remote version.", url));
                                progress
                                    .debug(&format!("*   {} is considered a remote change", url));
                                remote_item_changes.insert(url);
                            }
                        }
                        SyncStatus::LocallyDeleted(local_tag) => {
                            if &remote_tag == local_tag {
                                // This has been locally deleted
                                progress.debug(&format!("*   {} is a local deletion", url));
                                local_item_dels.insert(url);
                            } else {
                                progress.info(&format!("Conflict: task {} has been locally deleted and remotely modified. Reverting to the remote version.", url));
                                progress
                                    .debug(&format!("*   {} is a considered a remote change", url));
                                remote_item_changes.insert(url);
                            }
                        }
                    }
                }
            }
        }

        // Also iterate on the local tasks that are not on the remote
        for url in local_items_to_handle {
            progress.trace(&format!("##### Considering local item {}...", url));
            let local_item = match cal_local.get_item_by_url(&url).await {
                None => {
                    progress.error(&format!(
                        "Inconsistent state: missing task {} from the local tasks",
                        url
                    ));
                    continue;
                }
                Some(item) => item,
            };

            match local_item.sync_status() {
                SyncStatus::Synced(_) => {
                    // This item has been removed from the remote
                    //NOTE This implies "server supremacy"---the server is not a peer
                    progress.debug(&format!("#   {} is a deletion from the server", url));
                    remote_item_dels.insert(url);
                }
                SyncStatus::NotSynced => {
                    // This item has just been locally created
                    progress.debug(&format!("#   {} has been locally created", url));
                    local_item_additions.insert(url);
                }
                SyncStatus::LocallyDeleted(_) => {
                    // This item has been deleted from both sources
                    progress.debug(&format!("#   {} has been deleted from both sources", url));
                    remote_item_dels.insert(url);
                }
                SyncStatus::LocallyModified(_) => {
                    progress.info(&format!("Conflict: item {} has been deleted from the server and locally modified. Deleting the local copy", url));
                    remote_item_dels.insert(url);
                }
            }
        }

        Ok(ItemChanges {
            local_item_dels,
            remote_item_dels,
            local_item_changes,
            remote_item_changes,
            local_item_additions,
            remote_item_additions,
        })
    }

    /// Summarizes the delta between local and remote
    async fn calculate_prop_changes(
        cal_local: &T,
        cal_remote: &U,
        progress: &mut SyncProgress,
        cal_name: String,
    ) -> KFResult<PropChanges> {
        let mut local_prop_dels: HashSet<NamespacedName> = HashSet::new();
        let mut remote_prop_dels: HashSet<NamespacedName> = HashSet::new();
        let mut local_prop_changes: HashSet<NamespacedName> = HashSet::new();
        let mut remote_prop_changes: HashSet<Property> = HashSet::new();
        let mut local_prop_additions: HashSet<Property> = HashSet::new();
        let mut remote_prop_additions: HashSet<Property> = HashSet::new();

        let remote_props = cal_remote.get_properties().await?;

        progress.feedback(SyncEvent::PropsInProgress {
            calendar_name: cal_name.clone(),
            props_done_already: 0,
            details: format!("{} remote properties", remote_props.len()),
        });

        let mut local_props_to_handle: HashMap<NamespacedName, Property> = cal_local
            .get_properties()
            .await
            .values()
            .map(|p| (p.nsn().clone(), p.clone()))
            .collect();

        for remote_prop in remote_props {
            progress.trace(&format!("***** Considering remote prop {}...", remote_prop));
            match cal_local.get_property_by_name(remote_prop.nsn()).await {
                None => {
                    // This was created on the remote
                    progress.debug(&format!("*   {} is a remote addition", remote_prop));
                    remote_prop_additions.insert(remote_prop);
                }
                Some(local_prop) => {
                    debug_assert_eq!(remote_prop.nsn(), local_prop.nsn());
                    if local_props_to_handle.remove(remote_prop.nsn()).is_none() {
                        progress.error(&format!(
                            "Inconsistent state: missing prop {} from the local props",
                            remote_prop
                        ));
                    }

                    let prop_name: NamespacedName = local_prop.clone().into();

                    match local_prop.sync_status() {
                        SyncStatus::NotSynced => {
                            progress.error(&format!("Property reuse between remote and local sources ({}). Ignoring this item in the sync", prop_name));
                            continue;
                        }
                        SyncStatus::Synced(local_tag) => {
                            if remote_prop.value().as_str() != local_tag.as_str() {
                                // This has been modified on the remote
                                progress.debug(&format!("*   {} is a remote change", remote_prop));
                                remote_prop_changes.insert(remote_prop);
                            }
                        }
                        SyncStatus::LocallyModified(local_tag) => {
                            if remote_prop.value().as_str() == local_tag.as_str() {
                                // This has been changed locally
                                progress.debug(&format!("*   {} is a local change", local_prop));
                                local_prop_changes.insert(local_prop.nsn().clone());
                            } else {
                                progress.info(&format!("Conflict: prop {} has been modified in both sources. Using the remote version.", prop_name));
                                progress.debug(&format!(
                                    "*   {} is considered a remote change",
                                    remote_prop
                                ));
                                remote_prop_changes.insert(remote_prop);
                            }
                        }
                        SyncStatus::LocallyDeleted(local_tag) => {
                            if remote_prop.value().as_str() == local_tag.as_str() {
                                // This has been locally deleted
                                progress.debug(&format!("*   {} is a local deletion", remote_prop));
                                local_prop_dels.insert(prop_name);
                            } else {
                                progress.info(&format!("Conflict: prop {} has been locally deleted and remotely modified. Reverting to the remote version.", prop_name));
                                progress.debug(&format!(
                                    "*   {} is a considered a remote change",
                                    remote_prop
                                ));
                                remote_prop_changes.insert(remote_prop);
                            }
                        }
                    }
                }
            }
        }

        // Also iterate on the local props that are not on the remote
        for (prop_name, local_prop) in local_props_to_handle {
            debug_assert_eq!(&prop_name, local_prop.nsn());
            progress.trace(&format!("##### Considering local prop {}...", local_prop));
            match local_prop.sync_status() {
                SyncStatus::Synced(_) => {
                    // This item has been removed from the remote
                    //NOTE This implies "server supremacy"---the server is not a peer
                    progress.debug(&format!("#   {} is a deletion from the server", local_prop));
                    remote_prop_dels.insert(prop_name);
                }
                SyncStatus::NotSynced => {
                    // This item has just been locally created
                    progress.debug(&format!("#   {} has been locally created", local_prop));
                    local_prop_additions.insert(local_prop);
                }
                SyncStatus::LocallyDeleted(_) => {
                    // This item has been deleted from both sources
                    progress.debug(&format!(
                        "#   {} has been deleted from both sources",
                        local_prop
                    ));
                    remote_prop_dels.insert(prop_name);
                }
                SyncStatus::LocallyModified(_) => {
                    progress.info(&format!("Conflict: prop {} has been deleted from the server and locally modified. Deleting the local copy", prop_name));
                    remote_prop_dels.insert(prop_name);
                }
            }
        }

        Ok(PropChanges {
            local_prop_dels,
            remote_prop_dels,
            local_prop_changes,
            remote_prop_changes,
            local_prop_additions,
            remote_prop_additions,
        })
    }

    /// Based on the delta between local and remote, make whatever changes are necessary to bring the two sources into sync
    async fn commit_item_changes(
        cal_local: &mut T,
        cal_remote: &mut U,
        progress: &mut SyncProgress,
        cal_name: String,
        item_changes: ItemChanges,
    ) -> KFResult<()> {
        let ItemChanges {
            local_item_dels,
            remote_item_dels,
            local_item_changes,
            remote_item_changes,
            local_item_additions,
            remote_item_additions,
        } = item_changes;
        progress.trace("Committing changes to tasks...");
        for url_del in local_item_dels {
            progress.debug(&format!(
                "> Pushing local deletion {} to the server",
                url_del
            ));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::ItemsInProgress {
                calendar_name: cal_name.clone(),
                items_done_already: progress.counter(),
                details: Self::item_name(cal_local, &url_del).await,
            });

            match cal_remote.delete_item(&url_del).await {
                Err(err) => {
                    progress.warn(&format!(
                        "Unable to delete remote item {}: {}",
                        url_del, err
                    ));
                }
                Ok(()) => {
                    // Change the local copy from "marked to deletion" to "actually deleted"
                    if let Err(err) = cal_local.immediately_delete_item(&url_del).await {
                        progress.error(&format!(
                            "Unable to permanently delete local item {}: {}",
                            url_del, err
                        ));
                    }
                }
            }
        }

        for url_del in remote_item_dels {
            progress.debug(&format!("> Applying remote deletion {} locally", url_del));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::ItemsInProgress {
                calendar_name: cal_name.clone(),
                items_done_already: progress.counter(),
                details: Self::item_name(cal_local, &url_del).await,
            });
            if let Err(err) = cal_local.immediately_delete_item(&url_del).await {
                progress.warn(&format!("Unable to delete local item {}: {}", url_del, err));
            }
        }

        Self::apply_remote_item_additions(
            remote_item_additions,
            &mut *cal_local,
            &mut *cal_remote,
            progress,
            &cal_name,
        )
        .await;

        Self::apply_remote_item_changes(
            remote_item_changes,
            &mut *cal_local,
            &mut *cal_remote,
            progress,
            &cal_name,
        )
        .await;

        for url_add in local_item_additions {
            progress.debug(&format!(
                "> Pushing local addition {} to the server",
                url_add
            ));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::ItemsInProgress {
                calendar_name: cal_name.clone(),
                items_done_already: progress.counter(),
                details: Self::item_name(cal_local, &url_add).await,
            });
            match cal_local.get_item_by_url_mut(&url_add).await {
                None => {
                    progress.error(&format!("Inconsistency: created item {} has been marked for upload but is locally missing", url_add));
                    continue;
                }
                Some(item) => {
                    match cal_remote.add_item(item.clone()).await {
                        Err(err) => progress.error(&format!(
                            "Unable to add item {} to remote calendar: {}",
                            url_add, err
                        )),
                        Ok(new_ss) => {
                            // Update local sync status
                            item.set_sync_status(new_ss);
                        }
                    }
                }
            };
        }

        for url_change in local_item_changes {
            progress.debug(&format!(
                "> Pushing local change {} to the server",
                url_change
            ));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::ItemsInProgress {
                calendar_name: cal_name.clone(),
                items_done_already: progress.counter(),
                details: Self::item_name(cal_local, &url_change).await,
            });
            match cal_local.get_item_by_url_mut(&url_change).await {
                None => {
                    progress.error(&format!("Inconsistency: modified item {} has been marked for upload but is locally missing", url_change));
                    continue;
                }
                Some(item) => {
                    match cal_remote.update_item(item.clone()).await {
                        Err(err) => progress.error(&format!(
                            "Unable to update item {} in remote calendar: {}",
                            url_change, err
                        )),
                        Ok(new_ss) => {
                            // Update local sync status
                            item.set_sync_status(new_ss);
                        }
                    };
                }
            };
        }

        Ok(())
    }

    /// Based on the delta between local and remote, make whatever changes are necessary to bring the two sources into sync
    async fn commit_prop_changes(
        cal_local: &mut T,
        cal_remote: &mut U,
        progress: &mut SyncProgress,
        cal_name: String,
        prop_changes: PropChanges,
    ) -> KFResult<()> {
        log::debug!("committing prop changes: {:?}", prop_changes);
        let PropChanges {
            local_prop_dels,
            remote_prop_dels,
            local_prop_changes,
            remote_prop_changes,
            local_prop_additions,
            remote_prop_additions,
        } = prop_changes;
        progress.trace("Committing changes to props...");

        for prop_del in local_prop_dels {
            progress.debug(&format!(
                "> Pushing local prop deletion {} to the server",
                prop_del
            ));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::PropsInProgress {
                calendar_name: cal_name.clone(),
                props_done_already: progress.counter(),
                details: format!("{}", prop_del),
            });

            match cal_remote.delete_property(&prop_del).await {
                Err(err) => {
                    progress.warn(&format!(
                        "Unable to delete remote prop {}: {}",
                        prop_del, err
                    ));
                }
                Ok(()) => {
                    // Change the local copy from "marked to deletion" to "actually deleted"
                    if let Err(err) = cal_local.immediately_delete_prop(&prop_del).await {
                        progress.error(&format!(
                            "Unable to permanently delete local prop {}: {}",
                            prop_del, err
                        ));
                    }
                }
            }
        }

        for prop_del in remote_prop_dels {
            progress.debug(&format!("> Applying remote deletion {} locally", prop_del));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::PropsInProgress {
                calendar_name: cal_name.clone(),
                props_done_already: progress.counter(),
                details: format!("{}", prop_del),
            });
            if let Err(err) = cal_local.immediately_delete_prop(&prop_del).await {
                progress.warn(&format!(
                    "Unable to delete local prop {}: {}",
                    prop_del, err
                ));
            }
        }

        Self::apply_remote_prop_additions(
            remote_prop_additions,
            &mut *cal_local,
            progress,
            &cal_name,
        )
        .await;

        Self::apply_remote_prop_changes(remote_prop_changes, &mut *cal_local, progress, &cal_name)
            .await;

        for prop_add in local_prop_additions {
            progress.debug(&format!(
                "> Pushing local addition {} to the server",
                prop_add
            ));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::PropsInProgress {
                calendar_name: cal_name.clone(),
                props_done_already: progress.counter(),
                details: format!("{}", prop_add),
            });

            match cal_local.get_property_by_name_mut(prop_add.nsn()).await {
                None => {
                    progress.error(&format!("Inconsistency: created prop {} has been marked for upload but is locally missing", prop_add));
                    continue;
                }
                Some(local_prop) => {
                    match cal_remote.set_property(local_prop.clone()).await {
                        Err(err) => progress.error(&format!(
                            "Unable to add prop {} to remote calendar: {}",
                            prop_add, err
                        )),
                        Ok(ss) => {
                            // Update local sync status
                            local_prop.set_sync_status(ss);
                        }
                    }
                }
            };
        }

        for prop_change in local_prop_changes {
            progress.debug(&format!(
                "> Pushing local change {} to the server",
                prop_change
            ));
            progress.increment_counter(1);
            progress.feedback(SyncEvent::PropsInProgress {
                calendar_name: cal_name.clone(),
                props_done_already: progress.counter(),
                details: format!("{}", prop_change),
            });
            match cal_local.get_property_by_name_mut(&prop_change).await {
                None => {
                    progress.error(&format!("Inconsistency: modified prop {} has been marked for upload but is locally missing", prop_change));
                    continue;
                }
                Some(local_prop) => {
                    match cal_remote.set_property(local_prop.clone()).await {
                        Err(err) => progress.error(&format!(
                            "Unable to update prop {} in remote calendar: {}",
                            prop_change, err
                        )),
                        Ok(ss) => {
                            // Update local sync status
                            local_prop.set_sync_status(ss);
                        }
                    };
                }
            };
        }

        Ok(())
    }

    async fn item_name(cal: &T, url: &Url) -> String {
        cal.get_item_by_url(url)
            .await
            .map(|item| item.name())
            .unwrap_or_default()
            .to_string()
    }

    async fn apply_remote_item_additions(
        mut remote_additions: HashSet<Url>,
        cal_local: &mut T,
        cal_remote: &mut U,
        progress: &mut SyncProgress,
        cal_name: &str,
    ) {
        for batch in remote_additions
            .drain()
            .chunks(DOWNLOAD_BATCH_SIZE)
            .into_iter()
        {
            Self::fetch_batch_and_apply_items(
                BatchDownloadType::RemoteAdditions,
                batch,
                cal_local,
                cal_remote,
                progress,
                cal_name,
            )
            .await;
        }
    }

    async fn apply_remote_item_changes(
        mut remote_changes: HashSet<Url>,
        cal_local: &mut T,
        cal_remote: &mut U,
        progress: &mut SyncProgress,
        cal_name: &str,
    ) {
        for batch in remote_changes
            .drain()
            .chunks(DOWNLOAD_BATCH_SIZE)
            .into_iter()
        {
            Self::fetch_batch_and_apply_items(
                BatchDownloadType::RemoteChanges,
                batch,
                cal_local,
                cal_remote,
                progress,
                cal_name,
            )
            .await;
        }
    }

    async fn fetch_batch_and_apply_items<I: Iterator<Item = Url>>(
        batch_type: BatchDownloadType,
        remote_additions: I,
        cal_local: &mut T,
        cal_remote: &mut U,
        progress: &mut SyncProgress,
        cal_name: &str,
    ) {
        progress.debug(&format!("> Applying a batch of {} locally", batch_type) /* too bad Chunks does not implement ExactSizeIterator, that could provide useful debug info. See https://github.com/rust-itertools/itertools/issues/171 */);

        let list_of_additions: Vec<Url> = remote_additions.collect();
        match cal_remote.get_items_by_url(&list_of_additions).await {
            Err(err) => {
                progress.warn(&format!(
                    "Unable to get the batch of {} {:?}: {}. Skipping them.",
                    batch_type, list_of_additions, err
                ));
            }
            Ok(items) => {
                for item in items {
                    match item {
                        None => {
                            progress.error("Inconsistency: an item from the batch has vanished from the remote end");
                            continue;
                        }
                        Some(new_item) => {
                            let local_update_result = match batch_type {
                                BatchDownloadType::RemoteAdditions => {
                                    cal_local.add_item(new_item.clone()).await
                                }
                                BatchDownloadType::RemoteChanges => {
                                    cal_local.update_item(new_item.clone()).await
                                }
                            };
                            if let Err(err) = local_update_result {
                                progress.error(&format!(
                                    "Not able to add item {} to local calendar: {}",
                                    new_item.url(),
                                    err
                                ));
                            }
                        }
                    }
                }

                // Notifying every item at the same time would not make sense. Let's notify only one of them
                let one_item_name = match list_of_additions.first() {
                    Some(url) => Self::item_name(cal_local, url).await,
                    None => String::from("<unable to get the name of the first batched item>"),
                };
                progress.increment_counter(list_of_additions.len());
                progress.feedback(SyncEvent::ItemsInProgress {
                    calendar_name: cal_name.to_string(),
                    items_done_already: progress.counter(),
                    details: one_item_name,
                });
            }
        }
    }

    async fn apply_remote_prop_additions(
        mut remote_additions: HashSet<Property>,
        cal_local: &mut T,
        progress: &mut SyncProgress,
        cal_name: &str,
    ) {
        for batch in remote_additions
            .drain()
            .chunks(DOWNLOAD_BATCH_SIZE)
            .into_iter()
        {
            Self::fetch_batch_and_apply_props(
                BatchDownloadType::RemoteAdditions,
                batch,
                cal_local,
                progress,
                cal_name,
            )
            .await;
        }
    }

    async fn apply_remote_prop_changes(
        mut remote_changes: HashSet<Property>,
        cal_local: &mut T,
        progress: &mut SyncProgress,
        cal_name: &str,
    ) {
        for batch in remote_changes
            .drain()
            .chunks(DOWNLOAD_BATCH_SIZE)
            .into_iter()
        {
            Self::fetch_batch_and_apply_props(
                BatchDownloadType::RemoteChanges,
                batch,
                cal_local,
                progress,
                cal_name,
            )
            .await;
        }
    }

    async fn fetch_batch_and_apply_props<I: Iterator<Item = Property>>(
        batch_type: BatchDownloadType,
        remote_additions: I,
        cal_local: &mut T,
        progress: &mut SyncProgress,
        cal_name: &str,
    ) {
        progress.debug(&format!("> Applying a batch of {} locally", batch_type) /* too bad Chunks does not implement ExactSizeIterator, that could provide useful debug info. See https://github.com/rust-itertools/itertools/issues/171 */);
        let list_of_additions: Vec<Property> = remote_additions.collect();
        for new_prop in &list_of_additions {
            let synced_prop = {
                let mut p = new_prop.clone();

                // NOTE We mark the property's sync status as Synced with its own value as the version tag
                // See RemoteCalendar::set_property for more information on why
                p.mark_synced_to_self();

                p
            };
            let local_update_result = match batch_type {
                BatchDownloadType::RemoteAdditions => cal_local.add_property(synced_prop).await,
                BatchDownloadType::RemoteChanges => cal_local.update_property(synced_prop).await,
            };

            if let Err(err) = local_update_result {
                progress.error(&format!(
                    "Not able to add property {} to local calendar: {}",
                    new_prop, err
                ));
            }
        }

        // Notifying every prop at the same time would not make sense. Let's notify only one of them
        let one_prop_name = match list_of_additions.first() {
            Some(prop) => prop.to_string(),
            None => String::from("<unable to get the name of the first batched prop>"),
        };
        progress.increment_counter(list_of_additions.len());
        progress.feedback(SyncEvent::PropsInProgress {
            calendar_name: cal_name.to_string(),
            props_done_already: progress.counter(),
            details: one_prop_name,
        });
    }
}

async fn get_or_insert_counterpart_calendar<H, N, I>(
    haystack_descr: &str,
    haystack: &mut H,
    cal_url: &Url,
    needle: Arc<Mutex<N>>,
) -> KFResult<Arc<Mutex<I>>>
where
    H: CalDavSource<I>,
    I: BaseCalendar,
    N: BaseCalendar,
{
    loop {
        if let Some(cal) = haystack.get_calendar(cal_url).await {
            break Ok(cal);
        }

        // This calendar does not exist locally yet, let's add it
        log::debug!("Adding a {} calendar {}", haystack_descr, cal_url);
        let src = needle.lock().unwrap();
        let name = src.name().to_string();
        let supported_comps = src.supported_components();
        let color = src.color();
        haystack
            .create_calendar(cal_url.clone(), name, supported_comps, color.cloned())
            .await?;
    }
}
