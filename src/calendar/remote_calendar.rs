use std::collections::{HashMap, HashSet};
use std::error::Error;

use chrono::{DateTime, Utc};
use async_trait::async_trait;

use crate::traits::PartialCalendar;
use crate::calendar::SupportedComponents;
use crate::calendar::CalendarId;
use crate::item::ItemId;
use crate::item::Item;
use crate::resource::Resource;

static TASKS_BODY: &str = r#"
    <c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <d:getetag />
        </d:prop>
        <c:filter>
            <c:comp-filter name="VCALENDAR">
                <c:comp-filter name="VTODO" />
            </c:comp-filter>
        </c:filter>
    </c:calendar-query>
"#;




/// A CalDAV calendar created by a [`Client`](crate::client::Client).
#[derive(Clone)]
pub struct RemoteCalendar {
    name: String,
    resource: Resource,
    supported_components: SupportedComponents
}

impl RemoteCalendar {
    pub fn new(name: String, resource: Resource, supported_components: SupportedComponents) -> Self {
        Self {
            name, resource, supported_components,
        }
    }
}

#[async_trait]
impl PartialCalendar for RemoteCalendar {
    fn name(&self) -> &str { &self.name }
    fn id(&self) -> &CalendarId { &self.resource.url() }
    fn supported_components(&self) -> crate::calendar::SupportedComponents {
        self.supported_components
    }

    /// Returns the items that have been last-modified after `since`
    async fn get_items_modified_since(&self, since: Option<DateTime<Utc>>, _filter: Option<crate::calendar::SearchFilter>)
        -> Result<HashMap<ItemId, &Item>, Box<dyn Error>>
    {
        log::error!("Not implemented");
        Ok(HashMap::new())
    }

    /// Get the IDs of all current items in this calendar
    async fn get_item_ids(&self) -> Result<HashSet<ItemId>, Box<dyn Error>> {
        let responses = crate::client::sub_request_and_extract_elems(&self.resource, "REPORT", TASKS_BODY.to_string(), "response").await?;

        let mut item_ids = HashSet::new();
        for response in responses {
            let item_url = crate::utils::find_elem(&response, "href")
                .map(|elem| self.resource.combine(&elem.text()));

            match item_url {
                None => {
                    log::warn!("Unable to extract HREF");
                    continue;
                },
                Some(resource) => {
                    item_ids.insert(ItemId::from(&resource));
                },
            };
        }

        Ok(item_ids)
    }

    /// Returns a particular item
    async fn get_item_by_id_mut<'a>(&'a mut self, _id: &ItemId) -> Option<&'a mut Item> {
        log::error!("Not implemented");
        None
    }

    /// Add an item into this calendar
    async fn add_item(&mut self, _item: Item) {
        log::error!("Not implemented");
    }

    /// Remove an item from this calendar
    async fn delete_item(&mut self, _item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        log::error!("Not implemented");
        Ok(())
    }

}

