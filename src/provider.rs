//! This modules abstracts data sources and merges them in a single virtual one

use std::error::Error;

use chrono::{DateTime, Utc};

use crate::traits::CalDavSource;
use crate::Calendar;
use crate::Item;
use crate::item::ItemId;


pub struct Provider<S, L>
where
    S: CalDavSource,
    L: CalDavSource,
{
    /// The remote server
    server: S,
    /// The local cache
    local: L,

    /// The last time the provider successfully synchronized both sources
    last_sync: DateTime<Utc>,
}

impl<S,L> Provider<S, L>
where
    S: CalDavSource,
    L: CalDavSource,
{
    /// Create a provider that will merge both sources
    pub fn new(server: S, local: L, last_sync: DateTime<Utc>) -> Self {
        Self { server, local, last_sync }
    }

    pub fn server(&self) -> &S { &self.server }
    pub fn local(&self)  -> &L { &self.local }

    pub async fn sync(&mut self) -> Result<(), Box<dyn Error>> {
        let cals_server = self.server.get_calendars_mut().await?;

        for cal_server in cals_server {
            let cal_local = match self.local.get_calendar_mut(cal_server.url().clone()).await {
                None => {
                    log::error!("TODO: implement here");
                    continue;
                },
                Some(cal) => cal,
            };

            let server_mod = cal_server.get_tasks_modified_since(Some(self.last_sync));
            let server_del = cal_server.get_items_deleted_since(self.last_sync);
            let local_del = cal_local.get_items_deleted_since(self.last_sync);

            // Pull remote changes from the server
            let mut tasks_to_add_to_local = Vec::new();
            let mut tasks_id_to_remove_from_local = Vec::new();
            for deleted_id in server_del {
                tasks_id_to_remove_from_local.push(deleted_id);
            }
            for (new_id, new_item) in &server_mod {
                if server_mod.contains_key(new_id) {
                    log::warn!("Conflict for task {} ({}). Using the server version.", new_item.name(), new_id);
                    tasks_id_to_remove_from_local.push(new_id.clone());
                }
                tasks_to_add_to_local.push((*new_item).clone());
            }
            // Even in case of conflicts, "the server always wins", so it is safe to remove tasks from the local cache as soon as now
            remove_from_calendar(&tasks_id_to_remove_from_local, cal_local);



            // Push local changes to the server
            let local_mod = cal_local.get_tasks_modified_since(Some(self.last_sync));

            let mut tasks_to_add_to_server = Vec::new();
            let mut tasks_id_to_remove_from_server = Vec::new();
            for deleted_id in local_del {
                if server_mod.contains_key(&deleted_id) {
                    log::warn!("Conflict for task {}, that has been locally deleted and updated in the server. Using the server version.", deleted_id);
                    continue;
                }
                tasks_id_to_remove_from_server.push(deleted_id);
            }
            for (new_id, new_item) in &local_mod {
                if server_mod.contains_key(new_id) {
                    log::warn!("Conflict for task {} ({}). Using the server version.", new_item.name(), new_id);
                    continue;
                }
                tasks_to_add_to_server.push((*new_item).clone());
            }

            remove_from_calendar(&tasks_id_to_remove_from_server, cal_server);
            move_to_calendar(&mut tasks_to_add_to_local, cal_local);
            move_to_calendar(&mut tasks_to_add_to_server, cal_server);
        }

        Ok(())
    }
}


fn move_to_calendar(items: &mut Vec<Item>, calendar: &mut Calendar) {
    while items.len() > 0 {
        let item = items.remove(0);
        calendar.add_item(item);
    }
}

fn remove_from_calendar(ids: &Vec<ItemId>, calendar: &mut Calendar) {
    for id in ids {
        log::info!("  Removing {:?} from local calendar", id);
        calendar.delete_item(id);
    }
}
