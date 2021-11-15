//! This module provides a local cache for CalDAV data

use std::path::PathBuf;
use std::path::Path;
use std::error::Error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::ffi::OsStr;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use csscolorparser::Color;

use crate::traits::CalDavSource;
use crate::traits::BaseCalendar;
use crate::traits::CompleteCalendar;
use crate::calendar::cached_calendar::CachedCalendar;
use crate::calendar::CalendarId;
use crate::calendar::SupportedComponents;

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use crate::mock_behaviour::MockBehaviour;

const MAIN_FILE: &str = "data.json";

/// A CalDAV source that stores its items in a local folder.
///
/// It automatically updates the content of the folder when dropped (see its `Drop` implementation), but you can also manually call [`Cache::save_to_folder`]
///
/// Most of its methods are part of the `CalDavSource` trait implementation
#[derive(Debug)]
pub struct Cache {
    backing_folder: PathBuf,
    data: CachedData,

    /// In tests, we may add forced errors to this object
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    mock_behaviour: Option<Arc<Mutex<MockBehaviour>>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct CachedData {
    #[serde(skip)]
    calendars: HashMap<CalendarId, Arc<Mutex<CachedCalendar>>>,
}

impl Cache {
    /// Activate the "mocking remote source" features (i.e. tell its children calendars that they are mocked remote calendars)
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn set_mock_behaviour(&mut self, mock_behaviour: Option<Arc<Mutex<MockBehaviour>>>) {
        self.mock_behaviour = mock_behaviour;
    }


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

            #[cfg(feature = "local_calendar_mocks_remote_calendars")]
            mock_behaviour: None,
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

            #[cfg(feature = "local_calendar_mocks_remote_calendars")]
            mock_behaviour: None,
        }
    }

    /// Store the current Cache to its backing folder
    ///
    /// Note that this is automatically called when `self` is `drop`ped
    pub fn save_to_folder(&self) -> Result<(), std::io::Error> {
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


    /// Compares two Caches to check they have the same current content
    ///
    /// This is not a complete equality test: some attributes (sync status...) may differ. This should mostly be used in tests
    #[cfg(any(test, feature = "integration_tests"))]
    pub async fn has_same_observable_content_as(&self, other: &Self) -> Result<bool, Box<dyn Error>> {
        let calendars_l = self.get_calendars().await?;
        let calendars_r = other.get_calendars().await?;

        if crate::utils::keys_are_the_same(&calendars_l, &calendars_r) == false {
            log::debug!("Different keys for calendars");
            return Ok(false);
        }

        for (calendar_id, cal_l) in calendars_l {
            log::debug!("Comparing calendars {}", calendar_id);
            let cal_l = cal_l.lock().unwrap();
            let cal_r = match calendars_r.get(&calendar_id) {
                Some(c) => c.lock().unwrap(),
                None => return Err("should not happen, we've just tested keys are the same".into()),
            };

            // TODO: check calendars have the same names/ID/whatever
            if cal_l.has_same_observable_content_as(&cal_r).await? == false {
                log::debug!("Different calendars");
                return Ok(false)
            }

        }
        Ok(true)
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        if let Err(err) = self.save_to_folder() {
            log::error!("Unable to automatically save the cache when it's no longer required: {}", err);
        }
    }
}

impl Cache {
    /// The non-async version of [`crate::traits::CalDavSource::get_calendars`]
    pub fn get_calendars_sync(&self) -> Result<HashMap<CalendarId, Arc<Mutex<CachedCalendar>>>, Box<dyn Error>> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour.as_ref().map_or(Ok(()), |b| b.lock().unwrap().can_get_calendars())?;

        Ok(self.data.calendars.iter()
            .map(|(id, cal)| (id.clone(), cal.clone()))
            .collect()
        )
    }

    /// The non-async version of [`crate::traits::CalDavSource::get_calendar`]
    pub fn get_calendar_sync(&self, id: &CalendarId) -> Option<Arc<Mutex<CachedCalendar>>> {
        self.data.calendars.get(id).map(|arc| arc.clone())
    }
}

#[async_trait]
impl CalDavSource<CachedCalendar> for Cache {
    async fn get_calendars(&self) -> Result<HashMap<CalendarId, Arc<Mutex<CachedCalendar>>>, Box<dyn Error>> {
        self.get_calendars_sync()
    }

    async fn get_calendar(&self, id: &CalendarId) -> Option<Arc<Mutex<CachedCalendar>>> {
        self.get_calendar_sync(id)
    }

    async fn create_calendar(&mut self, id: CalendarId, name: String, supported_components: SupportedComponents, color: Option<Color>) -> Result<Arc<Mutex<CachedCalendar>>, Box<dyn Error>> {
        log::debug!("Inserting local calendar {}", id);
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour.as_ref().map_or(Ok(()), |b| b.lock().unwrap().can_create_calendar())?;

        let new_calendar = CachedCalendar::new(name, id.clone(), supported_components, color);
        let arc = Arc::new(Mutex::new(new_calendar));

        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        if let Some(behaviour) = &self.mock_behaviour {
            arc.lock().unwrap().set_mock_behaviour(Some(Arc::clone(behaviour)));
        };

        match self.data.calendars.insert(id, arc.clone()) {
            Some(_) => Err("Attempt to insert calendar failed: there is alredy such a calendar.".into()),
            None => Ok(arc),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use url::Url;
    use crate::calendar::SupportedComponents;
    use crate::item::Item;
    use crate::task::Task;

    async fn populate_cache(cache_path: &Path) -> Cache {
        let mut cache = Cache::new(&cache_path);

        let _shopping_list = cache.create_calendar(
            Url::parse("https://caldav.com/shopping").unwrap(),
            "My shopping list".to_string(),
            SupportedComponents::TODO,
            Some(csscolorparser::parse("lime").unwrap()),
        ).await.unwrap();

        let bucket_list = cache.create_calendar(
            Url::parse("https://caldav.com/bucket-list").unwrap(),
            "My bucket list".to_string(),
            SupportedComponents::TODO,
            Some(csscolorparser::parse("#ff8000").unwrap()),
        ).await.unwrap();

        {
            let mut bucket_list = bucket_list.lock().unwrap();
            let cal_id = bucket_list.id().clone();
            bucket_list.add_item(Item::Task(Task::new(
                String::from("Attend a concert of JS Bach"), false, &cal_id
            ))).await.unwrap();

            bucket_list.add_item(Item::Task(Task::new(
                String::from("Climb the Lighthouse of Alexandria"), true, &cal_id
            ))).await.unwrap();
        }

        cache
    }

    #[tokio::test]
    async fn cache_serde() {
        let _ = env_logger::builder().is_test(true).try_init();
        let cache_path = PathBuf::from(String::from("test_cache/serde_test"));
        let cache = populate_cache(&cache_path).await;

        cache.save_to_folder().unwrap();

        let retrieved_cache = Cache::from_folder(&cache_path).unwrap();
        assert_eq!(cache.backing_folder, retrieved_cache.backing_folder);
        let test = cache.has_same_observable_content_as(&retrieved_cache).await;
        println!("Equal? {:?}", test);
        assert_eq!(test.unwrap(), true);
    }

    #[tokio::test]
    async fn cache_sanity_checks() {
        let _ = env_logger::builder().is_test(true).try_init();
        let cache_path = PathBuf::from(String::from("test_cache/sanity_tests"));
        let mut cache = populate_cache(&cache_path).await;

        // We should not be able to add a second calendar with the same id
        let second_addition_same_calendar = cache.create_calendar(
            Url::parse("https://caldav.com/shopping").unwrap(),
            "My shopping list".to_string(),
            SupportedComponents::TODO,
            None,
        ).await;
        assert!(second_addition_same_calendar.is_err());
    }
}
