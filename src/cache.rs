//! This module provides a local cache for CalDAV data

use std::path::PathBuf;
use std::path::Path;
use std::error::Error;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::ffi::OsStr;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::traits::CalDavSource;
use crate::traits::PartialCalendar;
use crate::traits::CompleteCalendar;
use crate::calendar::cached_calendar::CachedCalendar;
use crate::calendar::CalendarId;

const MAIN_FILE: &str = "data.json";

/// A CalDAV source that stores its item in a local folder
#[derive(Debug)]
pub struct Cache {
    backing_folder: PathBuf,
    data: CachedData,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct CachedData {
    #[serde(skip)]
    calendars: HashMap<CalendarId, Arc<Mutex<CachedCalendar>>>,
}

impl Cache {
    /// Get the path to the cache folder
    pub fn cache_folder() -> PathBuf {
        return PathBuf::from(String::from("~/.config/my-tasks/cache/"))
    }

    /// Initialize a cache from the content of a valid backing folder if it exists.
    /// Returns an error otherwise
    pub fn from_folder(folder: &Path) -> Result<Self, Box<dyn Error>> {
        // Load shared data...
        let main_file = folder.join(MAIN_FILE);
        let mut data: CachedData = match std::fs::File::open(&main_file) {
            Err(err) => {
                return Err(format!("Unable to open file {:?}: {}", main_file, err).into());
            },
            Ok(file) => serde_json::from_reader(file)?,
        };

        // ...and every calendar
        for entry in std::fs::read_dir(folder)? {
            match entry {
                Err(err) => {
                    log::error!("Unable to read dir: {:?}", err);
                    continue;
                },
                Ok(entry) => {
                    let cal_path = entry.path();
                    log::debug!("Considering {:?}", cal_path);
                    if cal_path.extension() == Some(OsStr::new("cal")) {
                        match Self::load_calendar(&cal_path) {
                            Err(err) => {
                                log::error!("Unable to load calendar {:?} from cache: {:?}", cal_path, err);
                                continue;
                            },
                            Ok(cal) =>
                                data.calendars.insert(cal.id().clone(), Arc::new(Mutex::new(cal))),
                        };
                    }
                },
            }
        }

        Ok(Self{
            backing_folder: PathBuf::from(folder),
            data,
        })
    }

    fn load_calendar(path: &Path) -> Result<CachedCalendar, Box<dyn Error>> {
        let file = std::fs::File::open(&path)?;
        Ok(serde_json::from_reader(file)?)
    }

    /// Initialize a cache with the default contents
    pub fn new(folder_path: &Path) -> Self {
        Self{
            backing_folder: PathBuf::from(folder_path),
            data: CachedData::default(),
        }
    }

    /// Store the current Cache to its backing folder
    fn save_to_folder(&mut self) -> Result<(), std::io::Error> {
        let folder = &self.backing_folder;
        std::fs::create_dir_all(folder)?;

        // Save the general data
        let main_file_path = folder.join(MAIN_FILE);
        let file = std::fs::File::create(&main_file_path)?;
        serde_json::to_writer(file, &self.data)?;

        // Save each calendar
        for (cal_id, cal_mutex) in &self.data.calendars {
            let file_name = sanitize_filename::sanitize(cal_id.as_str()) + ".cal";
            let cal_file = folder.join(file_name);
            let file = std::fs::File::create(&cal_file)?;
            let cal = cal_mutex.lock().unwrap();
            serde_json::to_writer(file, &*cal)?;
        }
        Ok(())
    }


    pub fn add_calendar(&mut self, calendar: Arc<Mutex<CachedCalendar>>) {
        let id = calendar.lock().unwrap().id().clone();
        self.data.calendars.insert(id, calendar);
    }

    /// Compares two Caches to check they have the same current content
    ///
    /// This is not a complete equality test: some attributes (last sync date, deleted items...) may differ
    pub async fn has_same_contents_than(&self, other: &Self) -> Result<bool, Box<dyn Error>> {
        let calendars_l = self.get_calendars().await?;
        let calendars_r = other.get_calendars().await?;

        if keys_are_the_same(&calendars_l, &calendars_r) == false {
            log::debug!("Different keys for calendars");
            return Ok(false);
        }

        for (id, cal_l) in calendars_l {
            let cal_l = cal_l.lock().unwrap();
            let cal_r = match calendars_r.get(&id) {
                Some(c) => c.lock().unwrap(),
                None => return Err("should not happen, we've just tested keys are the same".into()),
            };

            let items_l = cal_l.get_items().await?;
            let items_r = cal_r.get_items().await?;

                    if keys_are_the_same(&items_l, &items_r) == false {
                log::debug!("Different keys for items");
                        return Ok(false);
}
                    for (id_l, item_l) in items_l {
                let item_r = match items_r.get(&id_l) {
                    Some(c) => c,
                    None => return Err("should not happen, we've just tested keys are the same".into()),
                };
                        if &item_l != item_r {
                    log::debug!("Different items");
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
    async fn get_calendars(&self) -> Result<HashMap<CalendarId, Arc<Mutex<CachedCalendar>>>, Box<dyn Error>> {
        Ok(self.data.calendars.iter()
            .map(|(id, cal)| (id.clone(), cal.clone()))
            .collect()
        )
    }

    async fn get_calendar(&self, id: &CalendarId) -> Option<Arc<Mutex<CachedCalendar>>> {
        self.data.calendars.get(id).map(|arc| arc.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use url::Url;
    use crate::calendar::SupportedComponents;

    #[tokio::test]
    async fn serde_cache() {
        let _ = env_logger::builder().is_test(true).try_init();

        let cache_path = PathBuf::from(String::from("test_cache/"));

        let mut cache = Cache::new(&cache_path);

        let cal1 = CachedCalendar::new("shopping list".to_string(),
                                Url::parse("https://caldav.com/shopping").unwrap(),
                            SupportedComponents::TODO);
        cache.add_calendar(Arc::new(Mutex::new(cal1)));


        cache.save_to_folder().unwrap();

        let retrieved_cache = Cache::from_folder(&cache_path).unwrap();
        assert_eq!(cache.backing_folder, retrieved_cache.backing_folder);
        let test = cache.has_same_contents_than(&retrieved_cache).await;
        println!("Equal? {:?}", test);
        assert_eq!(test.unwrap(), true);
    }
}
