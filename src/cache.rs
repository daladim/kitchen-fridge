//! This module provides a local cache for CalDAV data

use std::path::PathBuf;
use std::path::Path;
use std::error::Error;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use url::Url;
use chrono::{DateTime, Utc};

use crate::traits::CalDavSource;
use crate::traits::SyncSlave;
use crate::Calendar;


/// A CalDAV source that stores its item in a local file
#[derive(Debug, PartialEq)]
pub struct Cache {
    backing_file: PathBuf,
    data: CachedData,
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
struct CachedData {
    calendars: Vec<Calendar>,
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


    pub fn add_calendar(&mut self, calendar: Calendar) {
        self.data.calendars.push(calendar);
    }
}

#[async_trait]
impl CalDavSource for Cache {
    async fn get_calendars(&self) -> Result<&Vec<Calendar>, Box<dyn Error>> {
        Ok(&self.data.calendars)
    }

    async fn get_calendars_mut(&mut self) -> Result<Vec<&mut Calendar>, Box<dyn Error>> {
        Ok(
            self.data.calendars.iter_mut()
            .collect()
        )
    }

    async fn get_calendar(&self, url: Url) -> Option<&Calendar> {
        for cal in &self.data.calendars {
            if cal.url() == &url {
                return Some(cal);
}
        }
        return None;
    }
    async fn get_calendar_mut(&mut self, url: Url) -> Option<&mut Calendar> {
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

        let cal1 = Calendar::new("shopping list".to_string(),
                                Url::parse("https://caldav.com/shopping").unwrap(),
                            SupportedComponents::TODO);
        cache.add_calendar(cal1);

        cache.save_to_file();

        let retrieved_cache = Cache::from_file(&cache_path).unwrap();
        assert_eq!(cache, retrieved_cache);
    }
}
