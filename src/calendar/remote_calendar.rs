use std::collections::HashMap;
use std::error::Error;
use std::sync::Mutex;

use async_trait::async_trait;
use csscolorparser::Color;
use reqwest::{header::CONTENT_LENGTH, header::CONTENT_TYPE};
use url::Url;

use crate::calendar::SupportedComponents;
use crate::item::Item;
use crate::item::SyncStatus;
use crate::item::VersionTag;
use crate::resource::Resource;
use crate::traits::BaseCalendar;
use crate::traits::DavCalendar;
use crate::utils::find_elem;

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

static MULTIGET_BODY_PREFIX: &str = r#"
    <c:calendar-multiget xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
        <d:prop>
            <c:calendar-data />
        </d:prop>
"#;
static MULTIGET_BODY_SUFFIX: &str = r#"
    </c:calendar-multiget>
"#;

/// A CalDAV calendar created by a [`Client`](crate::client::Client).
#[derive(Debug)]
pub struct RemoteCalendar {
    name: String,
    resource: Resource,
    supported_components: SupportedComponents,
    color: Option<Color>,

    cached_version_tags: Mutex<Option<HashMap<Url, VersionTag>>>,
}

#[async_trait]
impl BaseCalendar for RemoteCalendar {
    fn name(&self) -> &str {
        &self.name
    }
    fn url(&self) -> &Url {
        &self.resource.url()
    }
    fn supported_components(&self) -> crate::calendar::SupportedComponents {
        self.supported_components
    }
    fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    async fn add_item(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        let ical_text = crate::ical::build_from(&item)?;

        let response = reqwest::Client::new()
            .put(item.url().clone())
            .header("If-None-Match", "*")
            .header(CONTENT_TYPE, "text/calendar")
            .header(CONTENT_LENGTH, ical_text.len())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(ical_text)
            .send()
            .await?;

        if response.status().is_success() == false {
            return Err(format!("Unexpected HTTP status code {:?}", response.status()).into());
        }

        let reply_hdrs = response.headers();
        match reply_hdrs.get("ETag") {
            None => Err(format!(
                "No ETag in these response headers: {:?} (request was {:?})",
                reply_hdrs,
                item.url()
            )
            .into()),
            Some(etag) => {
                let vtag_str = etag.to_str()?;
                let vtag = VersionTag::from(String::from(vtag_str));
                Ok(SyncStatus::Synced(vtag))
            }
        }
    }

    async fn update_item(&mut self, item: Item) -> Result<SyncStatus, Box<dyn Error>> {
        let old_etag = match item.sync_status() {
            SyncStatus::NotSynced => {
                return Err("Cannot update an item that has not been synced already".into())
            }
            SyncStatus::Synced(_) => {
                return Err("Cannot update an item that has not changed".into())
            }
            SyncStatus::LocallyModified(etag) => etag,
            SyncStatus::LocallyDeleted(etag) => etag,
        };
        let ical_text = crate::ical::build_from(&item)?;

        let request = reqwest::Client::new()
            .put(item.url().clone())
            .header("If-Match", old_etag.as_str())
            .header(CONTENT_TYPE, "text/calendar")
            .header(CONTENT_LENGTH, ical_text.len())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(ical_text)
            .send()
            .await?;

        if request.status().is_success() == false {
            return Err(format!("Unexpected HTTP status code {:?}", request.status()).into());
        }

        let reply_hdrs = request.headers();
        match reply_hdrs.get("ETag") {
            None => Err(format!(
                "No ETag in these response headers: {:?} (request was {:?})",
                reply_hdrs,
                item.url()
            )
            .into()),
            Some(etag) => {
                let vtag_str = etag.to_str()?;
                let vtag = VersionTag::from(String::from(vtag_str));
                Ok(SyncStatus::Synced(vtag))
            }
        }
    }
}

#[async_trait]
impl DavCalendar for RemoteCalendar {
    fn new(
        name: String,
        resource: Resource,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> Self {
        Self {
            name,
            resource,
            supported_components,
            color,
            cached_version_tags: Mutex::new(None),
        }
    }

    async fn get_item_version_tags(&self) -> Result<HashMap<Url, VersionTag>, Box<dyn Error>> {
        if let Some(map) = &*self.cached_version_tags.lock().unwrap() {
            log::debug!("Version tags are already cached.");
            return Ok(map.clone());
        };

        let responses = crate::client::sub_request_and_extract_elems(
            &self.resource,
            "REPORT",
            TASKS_BODY.to_string(),
            "response",
        )
        .await?;

        let mut items = HashMap::new();
        for response in responses {
            let item_url = crate::utils::find_elem(&response, "href")
                .map(|elem| self.resource.combine(&elem.text()));
            let item_url = match item_url {
                None => {
                    log::warn!("Unable to extract HREF");
                    continue;
                }
                Some(resource) => resource.url().clone(),
            };

            let version_tag = match crate::utils::find_elem(&response, "getetag") {
                None => {
                    log::warn!("Unable to extract ETAG for item {}, ignoring it", item_url);
                    continue;
                }
                Some(etag) => VersionTag::from(etag.text()),
            };

            items.insert(item_url.clone(), version_tag);
        }

        // Note: the mutex cannot be locked during this whole async function, but it can safely be re-entrant (this will just waste an unnecessary request)
        *self.cached_version_tags.lock().unwrap() = Some(items.clone());
        Ok(items)
    }

    async fn get_item_by_url(&self, url: &Url) -> Result<Option<Item>, Box<dyn Error>> {
        let res = reqwest::Client::new()
            .get(url.clone())
            .header(CONTENT_TYPE, "text/calendar")
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .send()
            .await?;

        if res.status().is_success() == false {
            return Err(format!("Unexpected HTTP status code {:?}", res.status()).into());
        }

        let text = res.text().await?;

        // This is supposed to be cached
        let version_tags = self.get_item_version_tags().await?;
        let vt = match version_tags.get(url) {
            None => return Err(format!("Inconsistent data: {} has no version tag", url).into()),
            Some(vt) => vt,
        };

        let item = crate::ical::parse(&text, url.clone(), SyncStatus::Synced(vt.clone()))?;
        Ok(Some(item))
    }

    async fn get_items_by_url(&self, urls: &[Url]) -> Result<Vec<Option<Item>>, Box<dyn Error>> {
        // Build the request body
        let mut hrefs = String::new();
        for url in urls {
            hrefs.push_str(&format!("        <d:href>{}</d:href>\n", url.path()));
        }
        let body = format!("{}{}{}", MULTIGET_BODY_PREFIX, hrefs, MULTIGET_BODY_SUFFIX);

        // Send the request
        let xml_replies = crate::client::sub_request_and_extract_elems(
            &self.resource,
            "REPORT",
            body,
            "response",
        )
        .await?;

        // This is supposed to be cached
        let version_tags = self.get_item_version_tags().await?;

        // Parse the results
        let mut results = Vec::new();
        for xml_reply in xml_replies {
            let href = find_elem(&xml_reply, "href").ok_or("Missing HREF")?.text();
            let mut url = self.resource.url().clone();
            url.set_path(&href);
            let ical_data = find_elem(&xml_reply, "calendar-data")
                .ok_or("Missing calendar-data")?
                .text();

            let vt = match version_tags.get(&url) {
                None => return Err(format!("Inconsistent data: {} has no version tag", url).into()),
                Some(vt) => vt,
            };

            let item = crate::ical::parse(&ical_data, url.clone(), SyncStatus::Synced(vt.clone()))?;
            results.push(Some(item));
        }

        Ok(results)
    }

    async fn delete_item(&mut self, item_url: &Url) -> Result<(), Box<dyn Error>> {
        let del_response = reqwest::Client::new()
            .delete(item_url.clone())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .send()
            .await?;

        if del_response.status().is_success() == false {
            return Err(format!("Unexpected HTTP status code {:?}", del_response.status()).into());
        }

        Ok(())
    }
}
