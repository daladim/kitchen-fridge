//! This module provides a local cache for CalDAV data

use std::path::PathBuf;
use std::path::Path;
use std::error::Error;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::traits::CalDavSource;
use crate::Calendar;


#[derive(Debug, PartialEq)]
pub struct Cache {
    backing_file: PathBuf,
    data: CachedData,
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
struct CachedData {
    calendars: Vec<Calendar>,
}

impl Cache {
    /// Get the cache file
    pub fn cache_file() -> PathBuf {
        return PathBuf::from(String::from("~/.config/my-tasks/cache.json"))
    }

    /// Initialize a cache from the content of a backing file (if it exists, otherwise start with the default contents)
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = match std::fs::File::open(path) {
            Err(_) => {
                CachedData::default()
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
}

impl Cache {
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
