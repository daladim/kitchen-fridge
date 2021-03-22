//! This modules abstracts data sources and merges them in a single virtual one

use std::error::Error;
use std::collections::HashSet;
use std::marker::PhantomData;

use chrono::{DateTime, Utc};

use crate::traits::{CalDavSource, CompleteCalendar};
use crate::traits::PartialCalendar;
use crate::Item;
use crate::item::ItemId;


/// A data source that combines two `CalDavSources` (usually a server and a local cache), which is able to sync both sources.
pub struct Provider<L, T, S, U>
where
    L: CalDavSource<T>,
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
    L: CalDavSource<T>,
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

    /// Performs a synchronisation between `local` and `server`.
    ///
    /// This bidirectional sync applies additions/deletions made on a source to the other source.
    /// In case of conflicts (the same item has been modified on both ends since the last sync, `server` always wins)
    pub async fn sync(&mut self) -> Result<(), Box<dyn Error>> {
        log::info!("Starting a sync.");

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

            // Step 1 - find the differences
            // let mut local_del = HashSet::new();
            // let mut remote_del = HashSet::new();
            // let mut local_changes = HashSet::new();
            // let mut remote_change = HashSet::new();
            // let mut local_additions = HashSet::new();
            // let mut remote_additions = HashSet::new();

        }

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
