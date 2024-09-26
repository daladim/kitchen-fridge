//! This module provides a local cache for CalDAV data

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use csscolorparser::Color;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use url::Url;

use crate::calendar::cached_calendar::CachedCalendar;
use crate::calendar::SupportedComponents;
use crate::error::KFError;
use crate::error::KFResult;
use crate::item::ItemType;
use crate::traits::BaseCalendar;
use crate::traits::CalDavSource;
use crate::traits::CompleteCalendar;

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use crate::mock_behaviour::MockBehaviour;

const MAIN_FILE: &str = "data.json";

#[derive(thiserror::Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error deserializing JSON: {0}")]
    JsonDeserializationError(#[from] serde_json::Error),

    #[error("Unable to open file {path:?}: {err}")]
    UnableToOpenFile { path: PathBuf, err: std::io::Error },
}

pub type CacheResult<T> = Result<T, CacheError>;

/// A CalDAV source that stores its items in a local folder.
///
/// It automatically updates the content of the folder when dropped (see its `Drop` implementation), but you can also manually call [`Cache::save_to_folder`]
///
/// Most of its functionality is provided by the `CalDavSource` async trait it implements.
/// However, since these functions do not _need_ to be actually async, non-async versions of them are also provided for better convenience. See [`Cache::get_calendar_sync`] for example
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
    calendars: HashMap<Url, Arc<Mutex<CachedCalendar>>>,
}

impl Cache {
    /// Activate the "mocking remote source" features (i.e. tell its children calendars that they are mocked remote calendars)
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn set_mock_behaviour(&mut self, mock_behaviour: Option<Arc<Mutex<MockBehaviour>>>) {
        self.mock_behaviour = mock_behaviour;
    }

    /// Get the path to the cache folder
    pub fn cache_folder() -> PathBuf {
        PathBuf::from(String::from("~/.config/my-tasks/cache/"))
    }

    /// Initialize a cache from the content of a valid backing folder if it exists.
    /// Returns an error otherwise
    pub fn from_folder(folder: &Path) -> CacheResult<Self> {
        // Load shared data...
        let main_file = folder.join(MAIN_FILE);
        let mut data: CachedData = match std::fs::File::open(&main_file) {
            Err(err) => {
                return Err(CacheError::UnableToOpenFile {
                    path: main_file,
                    err,
                });
            }
            Ok(file) => serde_json::from_reader(file)?,
        };

        // ...and every calendar
        for entry in std::fs::read_dir(folder)? {
            match entry {
                Err(err) => {
                    log::error!("Unable to read dir: {:?}", err);
                    continue;
                }
                Ok(entry) => {
                    let cal_path = entry.path();
                    log::debug!("Considering {:?}", cal_path);
                    if cal_path.extension() == Some(OsStr::new("cal")) {
                        match Self::load_calendar(&cal_path) {
                            Err(err) => {
                                log::error!(
                                    "Unable to load calendar {:?} from cache: {:?}",
                                    cal_path,
                                    err
                                );
                                continue;
                            }
                            Ok(cal) => data
                                .calendars
                                .insert(cal.url().clone(), Arc::new(Mutex::new(cal))),
                        };
                    }
                }
            }
        }

        Ok(Self {
            backing_folder: PathBuf::from(folder),
            data,

            #[cfg(feature = "local_calendar_mocks_remote_calendars")]
            mock_behaviour: None,
        })
    }

    fn load_calendar(path: &Path) -> CacheResult<CachedCalendar> {
        let file = std::fs::File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    }

    /// Initialize a cache with the default contents
    pub fn new(folder_path: &Path) -> Self {
        Self {
            backing_folder: PathBuf::from(folder_path),
            data: CachedData::default(),

            #[cfg(feature = "local_calendar_mocks_remote_calendars")]
            mock_behaviour: None,
        }
    }

    /// Store the current Cache to its backing folder
    ///
    /// Note that this is automatically called when `self` is `drop`ped
    pub async fn save_to_folder(&self) -> Result<(), std::io::Error> {
        let folder = &self.backing_folder;
        std::fs::create_dir_all(folder)?;

        // Save the general data
        let main_file_path = folder.join(MAIN_FILE);
        let file = std::fs::File::create(&main_file_path)?;
        serde_json::to_writer(file, &self.data)?;

        // Save each calendar
        for (cal_url, cal_mutex) in &self.data.calendars {
            let cal_file = self.calendar_path(cal_url);
            let file = std::fs::File::create(&cal_file)?;
            let cal = cal_mutex.lock().await;
            serde_json::to_writer(file, &*cal)?;
        }

        Ok(())
    }

    /// The path of the file where the calendar with the given URL is serialized
    pub fn calendar_path(&self, url: &Url) -> PathBuf {
        let file_name = sanitize_filename::sanitize(url.as_str()) + ".cal";
        self.backing_folder.join(file_name)
    }

    /// Compares two Caches to check they have the same current content
    ///
    /// This is not a complete equality test: some attributes (sync status...) may differ. This should mostly be used in tests
    #[cfg(any(test, feature = "integration_tests"))]
    pub async fn has_same_observable_content_as(
        &self,
        other: &Self,
        self_desc: &str,
        other_desc: &str,
    ) -> KFResult<bool> {
        let calendars_l = self.get_calendars().await?;
        let calendars_r = other.get_calendars().await?;

        if !crate::utils::keys_are_the_same(&calendars_l, &calendars_r) {
            log::debug!("Different keys for calendars");
            return Ok(false);
        }

        for (calendar_url, cal_l) in calendars_l {
            log::debug!("Comparing calendars {}", calendar_url);
            let cal_l = cal_l.lock().await;
            let cal_r = calendars_r
                .get(&calendar_url)
                .expect("should not happen, we've just tested keys are the same")
                .lock()
                .await;

            // TODO: check calendars have the same names/ID/whatever
            if !(cal_l
                .has_same_observable_content_as(&cal_r, self_desc, other_desc)
                .await?)
            {
                log::debug!("Different calendars");
                return Ok(false);
            }
        }
        Ok(true)
    }
}

// impl Default for Cache {
//     fn default() -> Self {
//         Self {
//             backing_folder: PathBuf::new(),
//             data: CachedData::default(),
//             #[cfg(feature = "local_calendar_mocks_remote_calendars")]
//             mock_behaviour: None,
//             dropped: false,
//         }
//     }
// }

// impl Drop for Cache {
//     fn drop(&mut self) {
//         if !self.dropped {
//             let mut this = Self::default();
//             std::mem::swap(&mut this, self);
//             this.dropped = true;
//             let fut = self.clone().save_to_folder();
//             tokio::spawn(async move {
//                 fut.await.unwrap();
//                 // if let Err(err) = self.save_to_folder().await {
//                 //     log::error!(
//                 //         "Unable to automatically save the cache when it's no longer required: {}",
//                 //         err
//                 //     );
//                 // }
//             });
//         }
//     }
// }

// #[async_trait]
// impl AsyncDrop for Cache {
//     async fn async_drop(&mut self) -> Result<(), AsyncDropError> {
//         if let Err(err) = self.save_to_folder().await {
//             log::error!(
//                 "Unable to automatically save the cache when it's no longer required: {}",
//                 err
//             );
//         }

//         Ok(())
//     }
// }

impl Cache {
    /// The non-async version of [`crate::traits::CalDavSource::get_calendars`]
    //FIXME misnomer
    pub async fn get_calendars_sync(&self) -> KFResult<HashMap<Url, Arc<Mutex<CachedCalendar>>>> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        if let Some(b) = self.mock_behaviour.as_ref() {
            b.lock().await.can_get_calendars()?;
        }

        Ok(self
            .data
            .calendars
            .iter()
            .map(|(url, cal)| (url.clone(), cal.clone()))
            .collect())
    }

    /// The non-async version of [`crate::traits::CalDavSource::get_calendar`]
    pub fn get_calendar_sync(&self, url: &Url) -> Option<Arc<Mutex<CachedCalendar>>> {
        self.data.calendars.get(url).cloned()
    }

    /// The non-async version of [`crate::traits::CalDavSource::delete_calendar`]
    pub fn delete_calendar_sync(
        &mut self,
        url: &Url,
    ) -> KFResult<Option<Arc<Mutex<CachedCalendar>>>> {
        // First, remove from filesystem
        let path = self.calendar_path(url);
        std::fs::remove_file(&path).map_err(|source| KFError::IoError {
            detail: format!("Could not remove calendar at path {}", path.display()),
            source,
        })?;

        // Then remove from memory
        match self.data.calendars.remove(url) {
            Some(c) => Ok(Some(c)),
            None => Err(KFError::ItemDoesNotExist {
                detail: "Can't delete calendar".into(),
                url: url.clone(),
                type_: Some(ItemType::Calendar),
            }),
        }
    }
}

#[async_trait]
impl CalDavSource<CachedCalendar> for Cache {
    async fn get_calendars(&self) -> KFResult<HashMap<Url, Arc<Mutex<CachedCalendar>>>> {
        self.get_calendars_sync().await
    }

    async fn get_calendar(&self, url: &Url) -> Option<Arc<Mutex<CachedCalendar>>> {
        self.get_calendar_sync(url)
    }

    async fn create_calendar(
        &mut self,
        url: Url,
        name: String,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> KFResult<Arc<Mutex<CachedCalendar>>> {
        log::debug!("Inserting local calendar {}", url);
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        if let Some(b) = self.mock_behaviour.as_ref() {
            b.lock().await.can_create_calendar()?;
        }

        let new_calendar = CachedCalendar::new(name, url.clone(), supported_components, color);
        let arc = Arc::new(Mutex::new(new_calendar));

        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        if let Some(behaviour) = &self.mock_behaviour {
            arc.lock()
                .await
                .set_mock_behaviour(Some(Arc::clone(behaviour)));
        };

        match self.data.calendars.insert(url.clone(), arc.clone()) {
            Some(_) => Err(KFError::ItemAlreadyExists {
                type_: ItemType::Calendar,
                detail: "Attempt to insert calendar failed".into(),
                url,
            }),
            None => Ok(arc),
        }
    }

    async fn delete_calendar(&mut self, url: &Url) -> KFResult<Option<Arc<Mutex<CachedCalendar>>>> {
        Self::delete_calendar_sync(self, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::calendar::SupportedComponents;
    use crate::item::Item;
    use crate::task::Task;
    use url::Url;

    async fn populate_cache(cache_path: &Path) -> Cache {
        let mut cache = Cache::new(cache_path);

        let _shopping_list = cache
            .create_calendar(
                Url::parse("https://caldav.com/shopping").unwrap(),
                "My shopping list".to_string(),
                SupportedComponents::TODO,
                Some(csscolorparser::parse("lime").unwrap()),
            )
            .await
            .unwrap();

        let bucket_list = cache
            .create_calendar(
                Url::parse("https://caldav.com/bucket-list").unwrap(),
                "My bucket list".to_string(),
                SupportedComponents::TODO,
                Some(csscolorparser::parse("#ff8000").unwrap()),
            )
            .await
            .unwrap();

        {
            let mut bucket_list = bucket_list.lock().await;
            let cal_url = bucket_list.url().clone();
            bucket_list
                .add_item(Item::Task(Task::new(
                    String::from("Attend a concert of JS Bach"),
                    false,
                    &cal_url,
                )))
                .await
                .unwrap();

            bucket_list
                .add_item(Item::Task(Task::new(
                    String::from("Climb the Lighthouse of Alexandria"),
                    true,
                    &cal_url,
                )))
                .await
                .unwrap();
        }

        cache
    }

    #[tokio::test]
    async fn cache_serde() {
        let _ = env_logger::builder().is_test(true).try_init();
        let cache_path = PathBuf::from(String::from("test_cache/serde_test"));
        let cache = populate_cache(&cache_path).await;

        cache.save_to_folder().await.unwrap();

        let retrieved_cache = Cache::from_folder(&cache_path).unwrap();
        assert_eq!(cache.backing_folder, retrieved_cache.backing_folder);
        let test = cache
            .has_same_observable_content_as(&retrieved_cache, "cache", "retrieved cache")
            .await;
        println!("Equal? {:?}", test);
        assert!(test.unwrap());
    }

    #[tokio::test]
    async fn cache_sanity_checks() {
        let _ = env_logger::builder().is_test(true).try_init();
        let cache_path = PathBuf::from(String::from("test_cache/sanity_tests"));
        let mut cache = populate_cache(&cache_path).await;

        // We should not be able to add a second calendar with the same URL
        let second_addition_same_calendar = cache
            .create_calendar(
                Url::parse("https://caldav.com/shopping").unwrap(),
                "My shopping list".to_string(),
                SupportedComponents::TODO,
                None,
            )
            .await;
        assert!(second_addition_same_calendar.is_err());
    }
}
