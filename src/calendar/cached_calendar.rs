use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use csscolorparser::Color;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::calendar::SupportedComponents;
use crate::error::KFError;
use crate::error::KFResult;
use crate::item::SyncStatus;
use crate::traits::{BaseCalendar, CompleteCalendar};
use crate::Item;

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use crate::mock_behaviour::MockBehaviour;
#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use std::sync::{Arc, Mutex};

/// A calendar used by the [`cache`](crate::cache) module
///
/// Most of its functionality is provided by the async traits it implements.
/// However, since these functions do not _need_ to be actually async, non-async versions of them are also provided for better convenience. See [`CachedCalendar::add_item_sync`] for example
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedCalendar {
    name: String,
    url: Url,
    supported_components: SupportedComponents,
    color: Option<Color>,
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    #[serde(skip)]
    mock_behaviour: Option<Arc<Mutex<MockBehaviour>>>,

    items: HashMap<Url, Item>,
}

impl CachedCalendar {
    /// Activate the "mocking remote calendar" feature (i.e. ignore sync statuses, since this is what an actual CalDAV sever would do)
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn set_mock_behaviour(&mut self, mock_behaviour: Option<Arc<Mutex<MockBehaviour>>>) {
        self.mock_behaviour = mock_behaviour;
    }

    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    fn add_item_maybe_mocked(&mut self, item: Item) -> KFResult<SyncStatus> {
        if self.mock_behaviour.is_some() {
            self.mock_behaviour
                .as_ref()
                .map_or(Ok(()), |b| b.lock().unwrap().can_add_item())?;
            Ok(self.add_or_update_item_force_synced(item))
        } else {
            Ok(self.regular_add_or_update_item(item))
        }
    }

    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    fn update_item_maybe_mocked(&mut self, item: Item) -> KFResult<SyncStatus> {
        if self.mock_behaviour.is_some() {
            self.mock_behaviour
                .as_ref()
                .map_or(Ok(()), |b| b.lock().unwrap().can_update_item())?;
            Ok(self.add_or_update_item_force_synced(item))
        } else {
            Ok(self.regular_add_or_update_item(item))
        }
    }

    /// Add or update an item
    fn regular_add_or_update_item(&mut self, item: Item) -> SyncStatus {
        let ss_clone = item.sync_status().clone();
        log::debug!("Adding or updating an item with {:?}", ss_clone);
        self.items.insert(item.url().clone(), item);
        ss_clone
    }

    /// Add or update an item, but force a "synced" SyncStatus. This is the normal behaviour that would happen on a server
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    fn add_or_update_item_force_synced(&mut self, mut item: Item) -> SyncStatus {
        log::debug!("Adding or updating an item, but forces a synced SyncStatus");
        match item.sync_status() {
            SyncStatus::Synced(_) => (),
            _ => item.set_sync_status(SyncStatus::random_synced()),
        };
        let ss_clone = item.sync_status().clone();
        self.items.insert(item.url().clone(), item);
        ss_clone
    }

    /// Some kind of equality check
    #[cfg(any(test, feature = "integration_tests"))]
    pub async fn has_same_observable_content_as(&self, other: &CachedCalendar) -> KFResult<bool> {
        if self.name != other.name
            || self.url != other.url
            || self.supported_components != other.supported_components
            || self.color != other.color
        {
            log::debug!("Calendar properties mismatch");
            return Ok(false);
        }

        let items_l = self.get_items().await?;
        let items_r = other.get_items().await?;

        if !crate::utils::keys_are_the_same(&items_l, &items_r) {
            log::debug!("Different keys for items");
            return Ok(false);
        }
        for (url_l, item_l) in items_l {
            let item_r = items_r
                .get(&url_l)
                .expect("should not happen, we've just tested keys are the same");
            if !item_l.has_same_observable_content_as(item_r) {
                log::debug!("Different items for URL {}:", url_l);
                log::debug!("{:#?}", item_l);
                log::debug!("{:#?}", item_r);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// The non-async version of [`Self::get_item_urls`]
    pub fn get_item_urls_sync(&self) -> HashSet<Url> {
        self.items.keys().cloned().collect()
    }

    /// The non-async version of [`Self::get_items`]
    pub fn get_items_sync(&self) -> HashMap<Url, &Item> {
        self.items
            .iter()
            .map(|(url, item)| (url.clone(), item))
            .collect()
    }

    /// The non-async version of [`Self::get_items_mut`]
    pub fn get_items_mut_sync(&mut self) -> HashMap<Url, &mut Item> {
        self.items
            .iter_mut()
            .map(|(url, item)| (url.clone(), item))
            .collect()
    }

    /// The non-async version of [`Self::get_item_by_url`]
    pub fn get_item_by_url_sync<'a>(&'a self, url: &Url) -> Option<&'a Item> {
        self.items.get(url)
    }

    /// The non-async version of [`Self::get_item_by_url_mut`]
    pub fn get_item_by_url_mut_sync<'a>(&'a mut self, url: &Url) -> Option<&'a mut Item> {
        self.items.get_mut(url)
    }

    /// The non-async version of [`Self::add_item`]
    pub fn add_item_sync(&mut self, item: Item) -> KFResult<SyncStatus> {
        if self.items.contains_key(item.url()) {
            return Err(KFError::ItemAlreadyExists {
                type_: item.type_(),
                detail: format!("Item {:?} cannot be added", item.url()),
                url: item.url().clone(),
            });
        }
        #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
        return Ok(self.regular_add_or_update_item(item));

        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        return self.add_item_maybe_mocked(item);
    }

    /// The non-async version of [`Self::update_item`]
    pub fn update_item_sync(&mut self, item: Item) -> KFResult<SyncStatus> {
        if !self.items.contains_key(item.url()) {
            return Err(KFError::ItemDoesNotExist {
                type_: Some(item.type_()),
                detail: "Item cannot be updated".into(),
                url: item.url().clone(),
            });
        }
        #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
        return Ok(self.regular_add_or_update_item(item));

        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        return self.update_item_maybe_mocked(item);
    }

    /// The non-async version of [`Self::mark_for_deletion`]
    pub fn mark_for_deletion_sync(&mut self, item_url: &Url) -> KFResult<()> {
        match self.items.get_mut(item_url) {
            None => Err(KFError::ItemDoesNotExist {
                type_: None,
                detail: "Can't mark item for deletion".into(),
                url: item_url.clone(),
            }),
            Some(item) => {
                match item.sync_status() {
                    SyncStatus::Synced(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status(SyncStatus::LocallyDeleted(prev_ss));
                    }
                    SyncStatus::LocallyModified(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status(SyncStatus::LocallyDeleted(prev_ss));
                    }
                    SyncStatus::LocallyDeleted(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status(SyncStatus::LocallyDeleted(prev_ss));
                    }
                    SyncStatus::NotSynced => {
                        // This was never synced to the server, we can safely delete it as soon as now
                        self.items.remove(item_url);
                    }
                };
                Ok(())
            }
        }
    }

    /// The non-async version of [`Self::immediately_delete_item`]
    pub fn immediately_delete_item_sync(&mut self, item_url: &Url) -> KFResult<()> {
        match self.items.remove(item_url) {
            None => Err(KFError::ItemDoesNotExist {
                type_: None,
                detail: "Can't immediately delete item".into(),
                url: item_url.clone(),
            }),
            Some(_) => Ok(()),
        }
    }

    pub fn set_name<S: ToString>(&mut self, name: S) {
        self.name = name.to_string();
    }
}

#[async_trait]
impl BaseCalendar for CachedCalendar {
    fn name(&self) -> &str {
        &self.name
    }

    fn url(&self) -> &Url {
        &self.url
    }

    fn supported_components(&self) -> SupportedComponents {
        self.supported_components
    }

    fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    async fn add_item(&mut self, item: Item) -> KFResult<SyncStatus> {
        self.add_item_sync(item)
    }

    async fn update_item(&mut self, item: Item) -> KFResult<SyncStatus> {
        self.update_item_sync(item)
    }
}

#[async_trait]
impl CompleteCalendar for CachedCalendar {
    fn new(
        name: String,
        url: Url,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> Self {
        Self {
            name,
            url,
            supported_components,
            color,
            #[cfg(feature = "local_calendar_mocks_remote_calendars")]
            mock_behaviour: None,
            items: HashMap::new(),
        }
    }

    async fn get_item_urls(&self) -> KFResult<HashSet<Url>> {
        Ok(self.get_item_urls_sync())
    }

    async fn get_items(&self) -> KFResult<HashMap<Url, &Item>> {
        Ok(self.get_items_sync())
    }

    async fn get_items_mut(&mut self) -> KFResult<HashMap<Url, &mut Item>> {
        Ok(self.get_items_mut_sync())
    }

    async fn get_item_by_url<'a>(&'a self, url: &Url) -> Option<&'a Item> {
        self.get_item_by_url_sync(url)
    }

    async fn get_item_by_url_mut<'a>(&'a mut self, url: &Url) -> Option<&'a mut Item> {
        self.get_item_by_url_mut_sync(url)
    }

    async fn mark_for_deletion(&mut self, item_url: &Url) -> KFResult<()> {
        self.mark_for_deletion_sync(item_url)
    }

    async fn immediately_delete_item(&mut self, item_url: &Url) -> KFResult<()> {
        self.immediately_delete_item_sync(item_url)
    }
}

// This class can be used to mock a remote calendar for integration tests

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use crate::{item::VersionTag, resource::Resource, traits::DavCalendar};

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
#[async_trait]
impl DavCalendar for CachedCalendar {
    fn new(
        name: String,
        resource: Resource,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> Self {
        crate::traits::CompleteCalendar::new(
            name,
            resource.url().clone(),
            supported_components,
            color,
        )
    }

    async fn get_item_version_tags(&self) -> KFResult<HashMap<Url, VersionTag>> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour
            .as_ref()
            .map_or(Ok(()), |b| b.lock().unwrap().can_get_item_version_tags())?;

        use crate::item::SyncStatus;

        let mut result = HashMap::new();

        for (url, item) in self.items.iter() {
            let vt = match item.sync_status() {
                SyncStatus::Synced(vt) => vt.clone(),
                _ => {
                    panic!(
                        "Mock calendars must contain only SyncStatus::Synced. Got {:?}",
                        item
                    );
                }
            };
            result.insert(url.clone(), vt);
        }

        Ok(result)
    }

    async fn get_item_by_url(&self, url: &Url) -> KFResult<Option<Item>> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour
            .as_ref()
            .map_or(Ok(()), |b| b.lock().unwrap().can_get_item_by_url())?;

        Ok(self.items.get(url).cloned())
    }

    async fn get_items_by_url(&self, urls: &[Url]) -> KFResult<Vec<Option<Item>>> {
        let mut v = Vec::new();
        for url in urls {
            v.push(DavCalendar::get_item_by_url(self, url).await?);
        }
        Ok(v)
    }

    async fn delete_item(&mut self, item_url: &Url) -> KFResult<()> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour
            .as_ref()
            .map_or(Ok(()), |b| b.lock().unwrap().can_delete_item())?;

        self.immediately_delete_item(item_url).await
    }
}
