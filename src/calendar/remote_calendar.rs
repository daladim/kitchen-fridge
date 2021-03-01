use crate::traits::PartialCalendar;

/// A CalDAV calendar created by a [`Client`](crate::client::Client).
pub struct RemoteCalendar {
    name: String,
    url: Url,
    supported_components: SupportedComponents
}

impl PartialCalendar for RemoteCalendar {
    fn name(&self) -> &str {
        &self.name
    }

    fn supported_components(&self) -> crate::calendar::SupportedComponents {
        self.supported_components
    }

    fn get_items_modified_since(&self, since: Option<DateTime<Utc>>, filter: Option<crate::calendar::SearchFilter>)
        -> HashMap<ItemId, &Item>
    {
        log::error!("Not implemented");
        HashMap::new()
    }

    fn get_item_by_id_mut(&mut self, id: &ItemId) -> Option<&mut Item> {
        log::error!("Not implemented");
        None
    }

    fn add_item(&mut self, item: Item) {
        log::error!("Not implemented");
    }

    fn delete_item(&mut self, item_id: &ItemId) {
        log::error!("Not implemented");
    }
}
