//! This module provides a local cache for CalDAV data

use std::path::PathBuf;
use std::path::Path;
use std::error::Error;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::traits::CalDavSource;
use crate::traits::SyncSlave;
use crate::traits::PartialCalendar;
use crate::traits::CompleteCalendar;
use crate::calendar::cached_calendar::CachedCalendar;
use crate::calendar::CalendarId;


/// A CalDAV source that stores its item in a local file
#[derive(Debug, PartialEq)]
pub struct Cache {
    backing_file: PathBuf,
    data: CachedData,
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
struct CachedData {
    calendars: HashMap<CalendarId, CachedCalendar>,
    last_sync: Option<DateTime<Utc>>,
}

impl Cache {
    /// Get the path to the cache file
    pub fn cache_file() -> PathBuf {
        return PathBuf::from(String::from("~/.config/my-tasks/cache.json"))
    }

    /// Initialize a cache from the content of a valid backing file if it exists.
    /// Returns an error otherwise
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = match std::fs::File::open(path) {
            Err(err) => {
                return Err(format!("Unable to open file {:?}: {}", path, err).into());
            },
            Ok(file) => serde_json::from_reader(file)?,
        };

        Ok(Self{
            backing_file: PathBuf::from(path),
            data,
        })
    }

    /// Initialize a cache with the default contents
    pub fn new(path: &Path) -> Self {
        Self{
            backing_file: PathBuf::from(path),
            data: CachedData::default(),
        }
    }

    /// Store the current Cache to its backing file
    fn save_to_file(&mut self) {
        // Save the contents to the file
        let path = &self.backing_file;
        let file = match std::fs::File::create(path) {
            Err(err) => {
                log::warn!("Unable to save file {:?}: {}", path, err);
                return;
            },
            Ok(f) => f,
        };

        if let Err(err) = serde_json::to_writer(file, &self.data) {
            log::warn!("Unable to serialize: {}", err);
            return;
        };
    }


    pub fn add_calendar(&mut self, calendar: CachedCalendar) {
        self.data.calendars.insert(calendar.id().clone(), calendar);
    }

    /// Compares two Caches to check they have the same current content
    ///
    /// This is not a complete equality test: some attributes (last sync date, deleted items...) may differ
    pub async fn has_same_contents_than(&self, other: &Self) -> Result<bool, Box<dyn Error>> {
        let calendars_l = self.get_calendars().await?;
        let calendars_r = other.get_calendars().await?;

        if keys_are_the_same(&calendars_l, &calendars_r) == false {
            return Ok(false);
        }

        for (id, cal_l) in calendars_l {
            let cal_r = match calendars_r.get(id) {
                Some(c) => c,
                None => return Err("should not happen, we've just tested keys are the same".into()),
            };

                    let items_l = cal_l.get_items();
                    let items_r = cal_r.get_items();

                    if keys_are_the_same(&items_l, &items_r) == false {
                        return Ok(false);
}
                    for (id_l, item_l) in items_l {
                let item_r = match items_r.get(&id_l) {
                    Some(c) => c,
                    None => return Err("should not happen, we've just tested keys are the same".into()),
                };
                        //println!("  items {} {}", item_r.name(), item_l.name());
                        if &item_l != item_r {
                            return Ok(false);
                        }
                    }
                }
        Ok(true)
    }
}

fn keys_are_the_same<T, U, V>(left: &HashMap<T, U>, right: &HashMap<T, V>) -> bool
where
    T: Hash + Eq + Clone,
{
    if left.len() != right.len() {
        return false;
    }

    let keys_l: HashSet<T> = left.keys().cloned().collect();
    let keys_r: HashSet<T> = right.keys().cloned().collect();
    keys_l == keys_r
}

#[async_trait]
impl CalDavSource<CachedCalendar> for Cache {
    async fn get_calendars(&self) -> Result<&HashMap<CalendarId, CachedCalendar>, Box<dyn Error>> {
        Ok(&self.data.calendars)
    }

    async fn get_calendars_mut(&mut self) -> Result<HashMap<CalendarId, &mut CachedCalendar>, Box<dyn Error>> {
        let mut hm = HashMap::new();
        for (id, val) in self.data.calendars.iter_mut() {
            hm.insert(id.clone(), val);
    }
        Ok(hm)
            }

    async fn get_calendar(&self, id: CalendarId) -> Option<&CachedCalendar> {
        self.data.calendars.get(&id)
        }
    async fn get_calendar_mut(&mut self, id: CalendarId) -> Option<&mut CachedCalendar> {
        self.data.calendars.get_mut(&id)
    }
}

impl SyncSlave for Cache {
    fn get_last_sync(&self) -> Option<DateTime<Utc>> {
        self.data.last_sync
    }

    fn update_last_sync(&mut self, timepoint: Option<DateTime<Utc>>) {
        self.data.last_sync = Some(timepoint.unwrap_or_else(|| Utc::now()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use url::Url;
    use crate::calendar::SupportedComponents;

    #[test]
    fn serde_cache() {
        let cache_path = PathBuf::from(String::from("cache.json"));

        let mut cache = Cache::new(&cache_path);

        let cal1 = CachedCalendar::new("shopping list".to_string(),
                                Url::parse("https://caldav.com/shopping").unwrap(),
                            SupportedComponents::TODO);
        cache.add_calendar(cal1);

        cache.save_to_file();

        let retrieved_cache = Cache::from_file(&cache_path).unwrap();
        assert_eq!(cache, retrieved_cache);
    }
}
