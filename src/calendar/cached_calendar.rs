use std::collections::{HashMap, HashSet};
use std::error::Error;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use csscolorparser::Color;

use crate::item::SyncStatus;
use crate::traits::{BaseCalendar, CompleteCalendar};
use crate::calendar::{CalendarId, SupportedComponents};
use crate::Item;
use crate::item::ItemId;

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use crate::mock_behaviour::MockBehaviour;


/// A calendar used by the [`cache`](crate::cache) module
///
/// Most of its methods are part of traits implementations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedCalendar {
    name: String,
    id: CalendarId,
    supported_components: SupportedComponents,
    color: Option<Color>,
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    #[serde(skip)]
    mock_behaviour: Option<Arc<Mutex<MockBehaviour>>>,

    items: HashMap<ItemId, Item>,
}

impl CachedCalendar {
    /// Activate the "mocking remote calendar" feature (i.e. ignore sync statuses, since this is what an actual CalDAV sever would do)
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn set_mock_behaviour(&mut self, mock_behaviour: Option<Arc<Mutex<MockBehaviour>>>) {
        self.mock_behaviour = mock_behaviour;
    }


    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    fn add_item_maybe_mocked(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        if self.mock_behaviour.is_some() {
            self.mock_behaviour.as_ref().map_or(Ok(()), |b| b.lock().unwrap().can_add_item())?;
            self.add_or_update_item_force_synced(item)
        } else {
            self.regular_add_or_update_item(item)
        }
    }

    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    fn update_item_maybe_mocked(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        if self.mock_behaviour.is_some() {
            self.mock_behaviour.as_ref().map_or(Ok(()), |b| b.lock().unwrap().can_update_item())?;
            self.add_or_update_item_force_synced(item)
        } else {
            self.regular_add_or_update_item(item)
        }
    }

    /// Add or update an item
    fn regular_add_or_update_item(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        let ss_clone = item.sync_status().clone();
        log::debug!("Adding or updating an item with {:?}", ss_clone);
        self.items.insert(item.id().clone(), item);
        Ok(ss_clone)
    }

    /// Add or update an item, but force a "synced" SyncStatus. This is the normal behaviour that would happen on a server
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    fn add_or_update_item_force_synced(&mut self, mut item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        log::debug!("Adding or updating an item, but forces a synced SyncStatus");
        match item.sync_status() {
            SyncStatus::Synced(_) => (),
            _ => item.set_sync_status(SyncStatus::random_synced()),
        };
        let ss_clone = item.sync_status().clone();
        self.items.insert(item.id().clone(), item);
        Ok(ss_clone)
    }

    /// Some kind of equality check
    #[cfg(any(test, feature = "integration_tests"))]
    pub async fn has_same_observable_content_as(&self, other: &CachedCalendar) -> Result<bool, Box<dyn Error>> {
        if self.name != other.name
        || self.id != other.id
        || self.supported_components != other.supported_components
        || self.color != other.color
        {
            log::debug!("Calendar properties mismatch");
            return Ok(false);
        }


        let items_l = self.get_items().await?;
        let items_r = other.get_items().await?;

        if crate::utils::keys_are_the_same(&items_l, &items_r) == false {
            log::debug!("Different keys for items");
            return Ok(false);
        }
        for (id_l, item_l) in items_l {
            let item_r = match items_r.get(&id_l) {
                Some(c) => c,
                None => return Err("should not happen, we've just tested keys are the same".into()),
            };
            if item_l.has_same_observable_content_as(&item_r) == false {
                log::debug!("Different items for id {}:", id_l);
                log::debug!("{:#?}", item_l);
                log::debug!("{:#?}", item_r);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// The non-async version of [`Self::get_item_ids`]
    pub fn get_item_ids_sync(&self) -> Result<HashSet<ItemId>, Box<dyn Error>> {
        Ok(self.items.iter()
            .map(|(id, _)| id.clone())
            .collect()
        )
    }

    /// The non-async version of [`Self::get_items`]
    pub fn get_items_sync(&self) -> Result<HashMap<ItemId, &Item>, Box<dyn Error>> {
        Ok(self.items.iter()
            .map(|(id, item)| (id.clone(), item))
            .collect()
        )
    }

    /// The non-async version of [`Self::get_item_by_id`]
    pub fn get_item_by_id_sync<'a>(&'a self, id: &ItemId) -> Option<&'a Item> {
        self.items.get(id)
    }

    /// The non-async version of [`Self::get_item_by_id_mut`]
    pub fn get_item_by_id_mut_sync<'a>(&'a mut self, id: &ItemId) -> Option<&'a mut Item> {
        self.items.get_mut(id)
    }

    /// The non-async version of [`Self::add_item`]
    pub fn add_item_sync(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        if self.items.contains_key(item.id()) {
            return Err(format!("Item {:?} cannot be added, it exists already", item.id()).into());
        }
        #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
        return self.regular_add_or_update_item(item);

        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        return self.add_item_maybe_mocked(item);
    }

    /// The non-async version of [`Self::update_item`]
    pub fn update_item_sync(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        if self.items.contains_key(item.id()) == false {
            return Err(format!("Item {:?} cannot be updated, it does not already exist", item.id()).into());
        }
        #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
        return self.regular_add_or_update_item(item);

        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        return self.update_item_maybe_mocked(item);
    }

    /// The non-async version of [`Self::mark_for_deletion`]
    pub fn mark_for_deletion_sync(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        match self.items.get_mut(item_id) {
            None => Err("no item for this key".into()),
            Some(item) => {
                match item.sync_status() {
                    SyncStatus::Synced(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status( SyncStatus::LocallyDeleted(prev_ss));
                    },
                    SyncStatus::LocallyModified(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status( SyncStatus::LocallyDeleted(prev_ss));
                    },
                    SyncStatus::LocallyDeleted(prev_ss) => {
                        let prev_ss = prev_ss.clone();
                        item.set_sync_status( SyncStatus::LocallyDeleted(prev_ss));
                    },
                    SyncStatus::NotSynced => {
                        // This was never synced to the server, we can safely delete it as soon as now
                        self.items.remove(item_id);
                    },
                };
                Ok(())
            }
        }
    }

    /// The non-async version of [`Self::immediately_delete_item`]
    pub fn immediately_delete_item_sync(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        match self.items.remove(item_id) {
            None => Err(format!("Item {} is absent from this calendar", item_id).into()),
            Some(_) => Ok(())
        }
    }

}


#[async_trait]
impl BaseCalendar for CachedCalendar {
    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &CalendarId {
        &self.id
    }

    fn supported_components(&self) -> SupportedComponents {
        self.supported_components
    }

    fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    async fn add_item(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        self.add_item_sync(item)
    }

    async fn update_item(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        self.update_item_sync(item)
    }
}

#[async_trait]
impl CompleteCalendar for CachedCalendar {
    fn new(name: String, id: CalendarId, supported_components: SupportedComponents, color: Option<Color>) -> Self {
        Self {
            name, id, supported_components, color,
            #[cfg(feature = "local_calendar_mocks_remote_calendars")]
            mock_behaviour: None,
            items: HashMap::new(),
        }
    }

    async fn get_item_ids(&self) -> Result<HashSet<ItemId>, Box<dyn Error>> {
        self.get_item_ids_sync()
    }

    async fn get_items(&self) -> Result<HashMap<ItemId, &Item>, Box<dyn Error>> {
        self.get_items_sync()
    }

    async fn get_item_by_id<'a>(&'a self, id: &ItemId) -> Option<&'a Item> {
        self.get_item_by_id_sync(id)
    }

    async fn get_item_by_id_mut<'a>(&'a mut self, id: &ItemId) -> Option<&'a mut Item> {
        self.get_item_by_id_mut_sync(id)
    }

    async fn mark_for_deletion(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        self.mark_for_deletion_sync(item_id)
    }

    async fn immediately_delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        self.immediately_delete_item_sync(item_id)
    }
}



// This class can be used to mock a remote calendar for integration tests

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use crate::{item::VersionTag,
            traits::DavCalendar,
            resource::Resource};

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
#[async_trait]
impl DavCalendar for CachedCalendar {
    fn new(name: String, resource: Resource, supported_components: SupportedComponents, color: Option<Color>) -> Self {
        crate::traits::CompleteCalendar::new(name, resource.url().clone(), supported_components, color)
    }

    async fn get_item_version_tags(&self) -> Result<HashMap<ItemId, VersionTag>, Box<dyn Error>> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour.as_ref().map_or(Ok(()), |b| b.lock().unwrap().can_get_item_version_tags())?;

        use crate::item::SyncStatus;

        let mut result = HashMap::new();

        for (id, item) in self.items.iter() {
            let vt = match item.sync_status() {
                SyncStatus::Synced(vt) => vt.clone(),
                _ => {
                    panic!("Mock calendars must contain only SyncStatus::Synced. Got {:?}", item);
                }
            };
            result.insert(id.clone(), vt);
        }

        Ok(result)
    }

    async fn get_item_by_id(&self, id: &ItemId) -> Result<Option<Item>, Box<dyn Error>> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour.as_ref().map_or(Ok(()), |b| b.lock().unwrap().can_get_item_by_id())?;

        Ok(self.items.get(id).cloned())
    }

    async fn delete_item(&mut self, item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        #[cfg(feature = "local_calendar_mocks_remote_calendars")]
        self.mock_behaviour.as_ref().map_or(Ok(()), |b| b.lock().unwrap().can_delete_item())?;

        self.immediately_delete_item(item_id).await
    }
}
