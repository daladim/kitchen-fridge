//! This modules abstracts data sources and merges them in a single virtual one

use std::error::Error;

use chrono::{DateTime, Utc};

use crate::traits::CalDavSource;
use crate::Calendar;
use crate::Task;
use crate::task::TaskId;


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

            let server_mod = cal_server.get_tasks_modified_since(Some(self.last_sync), None);
            let local_mod = cal_local.get_tasks_modified_since(Some(self.last_sync), None);

            let mut tasks_to_add_to_local = Vec::new();
            let mut tasks_id_to_remove_from_local = Vec::new();
            for (new_id, new_item) in &server_mod {
                if server_mod.contains_key(new_id) {
                    log::warn!("Conflict for task {} ({}). Using the server version.", new_item.name(), new_id);
                    tasks_id_to_remove_from_local.push(new_id.clone());
                }
                tasks_to_add_to_local.push((*new_item).clone());
            }

            let mut tasks_to_add_to_server = Vec::new();
            for (new_id, new_item) in &local_mod {
                if server_mod.contains_key(new_id) {
                    log::warn!("Conflict for task {} ({}). Using the server version.", new_item.name(), new_id);
                    continue;
                }
                tasks_to_add_to_server.push((*new_item).clone());
            }

            remove_from_calendar(&tasks_id_to_remove_from_local, cal_local);
            move_to_calendar(&mut tasks_to_add_to_local, cal_local);
            move_to_calendar(&mut tasks_to_add_to_server, cal_server);
        }

        Ok(())
    }
}


fn move_to_calendar(tasks: &mut Vec<Task>, calendar: &mut Calendar) {
    while tasks.len() > 0 {
        let task = tasks.remove(0);
        calendar.add_task(task);
    }
}

fn remove_from_calendar(ids: &Vec<TaskId>, calendar: &mut Calendar) {
    for id in ids {
        log::info!("  Removing {:?} from local calendar", id);
        calendar.delete_task(id);
    }
}
