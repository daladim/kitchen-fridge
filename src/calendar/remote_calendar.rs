use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;

use crate::traits::BaseCalendar;
use crate::traits::DavCalendar;
use crate::calendar::SupportedComponents;
use crate::calendar::CalendarId;
use crate::item::Item;
use crate::item::ItemId;
use crate::item::VersionTag;
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
impl BaseCalendar for RemoteCalendar {
    fn name(&self) -> &str { &self.name }
    fn id(&self) -> &CalendarId { &self.resource.url() }
    fn supported_components(&self) -> crate::calendar::SupportedComponents {
        self.supported_components
    }

    /// Add an item into this calendar
    async fn add_item(&mut self, _item: Item) -> Result<(), Box<dyn Error>> {
        Err("Not implemented".into())
    }

    async fn get_item_by_id<'a>(&'a self, id: &ItemId) -> Option<&'a Item> {
        log::error!("Not implemented");
        None
    }
}

#[async_trait]
impl DavCalendar for RemoteCalendar {
    async fn get_item_version_tags(&self) -> Result<HashMap<ItemId, VersionTag>, Box<dyn Error>> {
        let responses = crate::client::sub_request_and_extract_elems(&self.resource, "REPORT", TASKS_BODY.to_string(), "response").await?;

        let mut items = HashMap::new();
        for response in responses {
            let item_url = crate::utils::find_elem(&response, "href")
                .map(|elem| self.resource.combine(&elem.text()));
            let item_id = match item_url {
                None => {
                    log::warn!("Unable to extract HREF");
                    continue;
                },
                Some(resource) => {
                    ItemId::from(&resource)
                },
            };

            let version_tag = match crate::utils::find_elem(&response, "getetag") {
                None => {
                    log::warn!("Unable to extract ETAG for item {}, ignoring it", item_id);
                    continue;
                },
                Some(etag) => {
                    VersionTag::from(etag.text())
                }
            };

            items.insert(item_id, version_tag);
        }

        Ok(items)
    }

    async fn delete_item(&mut self, _item_id: &ItemId) -> Result<(), Box<dyn Error>> {
        log::error!("Not implemented");
        Ok(())
    }
}

