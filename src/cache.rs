//! This module provides a local cache for CalDAV data

use std::path::PathBuf;
use std::path::Path;
use std::error::Error;
use std::collections::HashMap;
use std::hash::Hash;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use url::Url;
use chrono::{DateTime, Utc};

use crate::traits::CalDavSource;
use crate::traits::SyncSlave;
use crate::traits::PartialCalendar;
use crate::traits::CompleteCalendar;
use crate::calendar::cached_calendar::CachedCalendar;


/// A CalDAV source that stores its item in a local file
#[derive(Debug, PartialEq)]
pub struct Cache {
    backing_file: PathBuf,
    data: CachedData,
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
struct CachedData {
    calendars: Vec<CachedCalendar>,
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
        self.data.calendars.push(calendar);
    }

    /// Compares two Caches to check they have the same current content
    ///
    /// This is not a complete equality test: some attributes (last sync date, deleted items...) may differ
    pub async fn has_same_contents_than(&self, other: &Self) -> Result<bool, Box<dyn Error>> {
        let calendars_l = self.get_calendars().await?;
        let calendars_r = other.get_calendars().await?;

        for cal_l in calendars_l {
            for cal_r in calendars_r {
                if cal_l.url() == cal_r.url() {
                    let items_l = cal_l.get_items();
                    let items_r = cal_r.get_items();

                    if keys_are_the_same(&items_l, &items_r) == false {
                        return Ok(false);
}
                    for (id_l, item_l) in items_l {
                        let item_r = items_r.get(&id_l).unwrap();
                        //println!("  items {} {}", item_r.name(), item_l.name());
                        if &item_l != item_r {
                            return Ok(false);
                        }
                    }

                    break;
                }
            }
        }
        // TODO: also check there is no missing calendar in one side or the other
        // TODO: also check there is no missing task in either side of each calendar
        Ok(true)
    }
}

fn keys_are_the_same<T, U, V>(left: &HashMap<T, U>, right: &HashMap<T, V>) -> bool
where
    T: Hash + Eq,
{
    left.len() == right.len()
    && left.keys().all(|k| right.contains_key(k))
}

#[async_trait]
impl CalDavSource<CachedCalendar> for Cache {
    async fn get_calendars(&self) -> Result<&Vec<CachedCalendar>, Box<dyn Error>> {
        Ok(&self.data.calendars)
    }

    async fn get_calendars_mut(&mut self) -> Result<Vec<&mut CachedCalendar>, Box<dyn Error>> {
        Ok(
            self.data.calendars.iter_mut()
            .collect()
        )
    }

    async fn get_calendar(&self, url: Url) -> Option<&CachedCalendar> {
        for cal in &self.data.calendars {
            if cal.url() == &url {
                return Some(cal);
            }
        }
        return None;
    }
    async fn get_calendar_mut(&mut self, url: Url) -> Option<&mut CachedCalendar> {
        for cal in &mut self.data.calendars {
            if cal.url() == &url {
                return Some(cal);
            }
        }
        return None;
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
