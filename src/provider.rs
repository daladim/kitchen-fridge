//! This modules abstracts data sources and merges them in a single virtual one

use std::error::Error;
use std::collections::HashSet;
use std::marker::PhantomData;

use chrono::{DateTime, Utc};

use crate::traits::{CalDavSource, CompleteCalendar};
use crate::traits::SyncSlave;
use crate::traits::PartialCalendar;
use crate::Item;
use crate::item::ItemId;


/// A data source that combines two `CalDavSources` (usually a server and a local cache), which is able to sync both sources.
pub struct Provider<L, T, S, U>
where
    L: CalDavSource<T> + SyncSlave,
    T: CompleteCalendar,
    S: CalDavSource<U>,
    U: PartialCalendar + Sync + Send,
{
    /// The remote server
    server: S,
    /// The local cache
    local: L,

    phantom_t: PhantomData<T>,
    phantom_u: PhantomData<U>,
}

impl<L, T, S, U> Provider<L, T, S, U>
where
    L: CalDavSource<T> + SyncSlave,
    T: CompleteCalendar,
    S: CalDavSource<U>,
    U: PartialCalendar + Sync + Send,
{
    /// Create a provider.
    ///
    /// `server` is usually a [`Client`](crate::client::Client), `local` is usually a [`Cache`](crate::cache::Cache).
    /// However, both can be interchangeable. The only difference is that `server` always wins in case of a sync conflict
    pub fn new(server: S, local: L) -> Self {
        Self { server, local,
            phantom_t: PhantomData, phantom_u: PhantomData,
        }
    }

    /// Returns the data source described as the `server`
    pub fn server(&self) -> &S { &self.server }
    /// Returns the data source described as the `local`
    pub fn local(&self)  -> &L { &self.local }
    /// Returns the last time the `local` source has been synced
    pub fn last_sync_timestamp(&self) -> Option<DateTime<Utc>> {
        self.local.get_last_sync()
    }

    /// Performs a synchronisation between `local` and `server`.
    ///
    /// This bidirectional sync applies additions/deletions made on a source to the other source.
    /// In case of conflicts (the same item has been modified on both ends since the last sync, `server` always wins)
    pub async fn sync(&mut self) -> Result<(), Box<dyn Error>> {
        let last_sync = self.local.get_last_sync();
        log::info!("Starting a sync. Last sync was at {:?}", last_sync);

        let cals_server = self.server.get_calendars().await?;
        for (id, cal_server) in cals_server {
            let mut cal_server = cal_server.lock().unwrap();

            let cal_local = match self.local.get_calendar(&id).await {
                None => {
                    log::error!("TODO: implement here");
                    continue;
                },
                Some(cal) => cal,
            };
            let mut cal_local = cal_local.lock().unwrap();

            // Step 1 - "Server always wins", so a delteion from the server must be applied locally, even if it was locally modified.
            let mut local_dels = match last_sync {
                None => HashSet::new(),
                Some(date) => cal_local.get_items_deleted_since(date).await,
            };
            if last_sync.is_some() {
                let server_deletions = cal_server.find_deletions_from(cal_local.get_item_ids().await).await;
                for server_del_id in server_deletions {
                    // Even in case of conflicts, "the server always wins", so it is safe to remove tasks from the local cache as soon as now
                    if let Err(err) = cal_local.delete_item(&server_del_id).await {
                        log::error!("Unable to remove local item {}: {}", server_del_id, err);
                    }

                    if local_dels.contains(&server_del_id) {
                        local_dels.remove(&server_del_id);
                }
            }
            }

            // Step 2 - Compare both changesets...
            let server_mods = cal_server.get_items_modified_since(last_sync, None).await;
            let mut local_mods = cal_local.get_items_modified_since(last_sync, None).await;

            // ...import remote changes,...
            let mut conflicting_tasks = Vec::new();
            let mut tasks_to_add = Vec::new();
            for (server_mod_id, server_mod) in server_mods {
                if local_mods.contains_key(&server_mod_id) {
                    log::warn!("Conflict for task {} (modified in both sources). Using the server version", server_mod_id);
                    conflicting_tasks.push(server_mod_id.clone());
                    local_mods.remove(&server_mod_id);
                }
                if local_dels.contains(&server_mod_id) {
                    log::warn!("Conflict for task {} (modified in the server, deleted locally). Reverting to the server version", server_mod_id);
                    local_dels.remove(&server_mod_id);
                }
                tasks_to_add.push(server_mod.clone());
            }

            // ...upload local deletions,...
            for local_del_id in local_dels {
                if let Err(err) = cal_server.delete_item(&local_del_id).await {
                    log::error!("Unable to remove remote item {}: {}", local_del_id, err);
                }
            }

            // ...and upload local changes
            for (local_mod_id, local_mod) in local_mods {
                // Conflicts are no longer in local_mods
                if let Err(err) = cal_server.delete_item(&local_mod_id).await {
                    log::error!("Unable to remove remote item (before an update) {}: {}", local_mod_id, err);
                }
                // TODO: should I add a .update_item()?
                cal_server.add_item(local_mod.clone()).await;
            }

            remove_from_calendar(&conflicting_tasks, &mut (*cal_local)).await;
            move_to_calendar(&mut tasks_to_add, &mut (*cal_local)).await;
        }

        self.local.update_last_sync(None);

        Ok(())
    }
}


async fn move_to_calendar<C: PartialCalendar>(items: &mut Vec<Item>, calendar: &mut C) {
    while items.len() > 0 {
        let item = items.remove(0);
        log::warn!("Moving {} to calendar", item.name());
        calendar.add_item(item).await;
    }
}

async fn remove_from_calendar<C: PartialCalendar>(ids: &Vec<ItemId>, calendar: &mut C) {
    for id in ids {
        log::info!("  Removing {:?} from calendar", id);
        if let Err(err) = calendar.delete_item(id).await {
            log::warn!("Unable to delete item {:?} from calendar: {}", id, err);
        }
    }
}
