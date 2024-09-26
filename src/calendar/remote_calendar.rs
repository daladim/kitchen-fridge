use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use csscolorparser::Color;
use http::header::ToStrError;
use http::{HeaderValue, Method};
use reqwest::header::HeaderMap;
use reqwest::{header::CONTENT_LENGTH, header::CONTENT_TYPE};
use url::Url;

use crate::calendar::SupportedComponents;
use crate::error::{HttpStatusConstraint, KFError, KFResult};
use crate::item::Item;
use crate::resource::Resource;
use crate::traits::BaseCalendar;
use crate::traits::DavCalendar;
use crate::utils::prop::{Property, PROP_ALLPROP};
use crate::utils::req::{propfind_body, sub_request_and_extract_elems};
use crate::utils::sync::{SyncStatus, VersionTag};
use crate::utils::xml::find_elem;
use crate::utils::NamespacedName;

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

#[derive(thiserror::Error, Debug)]
pub enum RemoteCalendarError {
    #[error("Cannot update an item that has not been synced already")]
    CannotUpdateUnsyncedItem,

    #[error("Cannot update an item that has not changed")]
    CannotUpdateUnchangedItem,

    #[error("Non-ASCII header: {header:?}: {source}")]
    NonAsciiHeader {
        header: HeaderValue,
        source: ToStrError,
    },

    #[error("Inconsistent data: {0} has no version tag")]
    ItemLacksVersionTag(Url),

    #[error("No ETag in these response headers: {response_headers:?} (request was {url:?})")]
    NoETag {
        url: Url,
        response_headers: HeaderMap,
    },
}

/// A CalDAV calendar created by a [`Client`](crate::client::Client).
#[derive(Debug)]
pub struct RemoteCalendar {
    name: String,
    resource: Resource,
    supported_components: SupportedComponents,
    color: Option<Color>,

    cached_version_tags: Mutex<Option<HashMap<Url, VersionTag>>>,
}

impl RemoteCalendar {
    async fn get_properties(&self, props: &[NamespacedName]) -> KFResult<Vec<Property>> {
        let body = propfind_body(props);
        let propstats =
            sub_request_and_extract_elems(&self.resource, "PROPFIND", body, 0, "propstat").await?;

        let mut props = Vec::new();
        for propstat in propstats {
            if let Some(prop_el) = find_elem(&propstat, "prop") {
                for child in prop_el.children() {
                    props.push(Property::new(child.ns(), child.name(), child.text()));
                }
            } else {
                return Err(KFError::MissingDOMElement {
                    text: propstat.text(),
                    el: "prop".to_string(),
                });
            }
        }

        Ok(props)
    }
}

#[async_trait]
impl BaseCalendar for RemoteCalendar {
    fn name(&self) -> &str {
        &self.name
    }
    fn url(&self) -> &Url {
        self.resource.url()
    }
    fn supported_components(&self) -> crate::calendar::SupportedComponents {
        self.supported_components
    }
    fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    async fn get_properties_by_name(
        &self,
        names: &[NamespacedName],
    ) -> KFResult<Vec<Option<Property>>> {
        self.get_properties(names).await.map(|props| {
            names
                .iter()
                .map(|n| props.iter().find(|p| p.nsn() == n).cloned())
                .collect()
        })
    }

    async fn set_property(&mut self, prop: Property) -> KFResult<SyncStatus> {
        let method: Method = "PROPPATCH".parse().expect("invalid method name");
        let url = self.url().clone();

        let propertyupdate = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
     <D:propertyupdate xmlns:D="DAV:" xmlns:A="{}">
       <D:set>
         <D:prop>
             <A:{}>{}</A:{}>
         </D:prop>
       </D:set>
     </D:propertyupdate>"#,
            prop.xmlns(),
            prop.name(),
            prop.value(),
            prop.name()
        );

        let response = Box::pin(reqwest::Client::new())
            .request(method.clone(), url.clone())
            .header(CONTENT_TYPE, "application/xml")
            .header(CONTENT_LENGTH, propertyupdate.len())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(propertyupdate)
            .send()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url,
                method,
                source,
            })?;

        if !response.status().is_success() {
            return Err(KFError::UnexpectedHTTPStatusCode {
                expected: HttpStatusConstraint::Success,
                got: response.status(),
            });
        }

        // We use the property value itself, rather than a server-generated etag, because it fully captures its own content
        // This saves us a PROPFIND to query the etag
        // If property values ever get too large, we may have to change the approach
        Ok(SyncStatus::Synced(VersionTag::from(prop.value().clone())))
    }

    async fn add_item(&mut self, item: Item) -> KFResult<SyncStatus> {
        let ical_text = crate::ical::build_from(&item);

        let response = reqwest::Client::new()
            .put(item.url().clone())
            .header("If-None-Match", "*")
            .header(CONTENT_TYPE, "text/calendar")
            .header(CONTENT_LENGTH, ical_text.len())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(ical_text)
            .send()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url: item.url().clone(),
                method: Method::GET,
                source,
            })?;

        if !response.status().is_success() {
            return Err(KFError::UnexpectedHTTPStatusCode {
                expected: HttpStatusConstraint::Success,
                got: response.status(),
            });
        }

        let reply_hdrs = response.headers();
        match reply_hdrs.get("ETag") {
            None => Err(RemoteCalendarError::NoETag {
                url: item.url().clone(),
                response_headers: reply_hdrs.clone(),
            }
            .into()),
            Some(etag) => {
                let vtag_str =
                    etag.to_str()
                        .map_err(|source| RemoteCalendarError::NonAsciiHeader {
                            header: etag.clone(),
                            source,
                        })?;
                let vtag = VersionTag::from(String::from(vtag_str));
                Ok(SyncStatus::Synced(vtag))
            }
        }
    }

    async fn update_item(&mut self, item: Item) -> KFResult<SyncStatus> {
        let old_etag = match item.sync_status() {
            SyncStatus::NotSynced => {
                return Err(RemoteCalendarError::CannotUpdateUnsyncedItem.into())
            }
            SyncStatus::Synced(_) => {
                return Err(RemoteCalendarError::CannotUpdateUnchangedItem.into())
            }
            SyncStatus::LocallyModified(etag) => etag,
            SyncStatus::LocallyDeleted(etag) => etag,
        };
        let ical_text = crate::ical::build_from(&item);

        let request = reqwest::Client::new()
            .put(item.url().clone())
            .header("If-Match", old_etag.as_str())
            .header(CONTENT_TYPE, "text/calendar")
            .header(CONTENT_LENGTH, ical_text.len())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(ical_text)
            .send()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url: item.url().clone(),
                method: Method::PUT,
                source,
            })?;

        if !request.status().is_success() {
            return Err(KFError::UnexpectedHTTPStatusCode {
                expected: HttpStatusConstraint::Success,
                got: request.status(),
            });
        }

        let reply_hdrs = request.headers();
        match reply_hdrs.get("ETag") {
            None => Err(RemoteCalendarError::NoETag {
                url: item.url().clone(),
                response_headers: reply_hdrs.clone(),
            }
            .into()),
            Some(etag) => {
                let vtag_str =
                    etag.to_str()
                        .map_err(|source| RemoteCalendarError::NonAsciiHeader {
                            header: etag.clone(),
                            source,
                        })?;
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

    async fn get_item_version_tags(&self) -> KFResult<HashMap<Url, VersionTag>> {
        if let Some(map) = &*self.cached_version_tags.lock().unwrap() {
            log::debug!("Version tags are already cached.");
            return Ok(map.clone());
        };

        let responses = sub_request_and_extract_elems(
            &self.resource,
            "REPORT",
            TASKS_BODY.to_string(),
            1,
            "response",
        )
        .await?;

        let mut items = HashMap::new();
        for response in responses {
            let item_url =
                find_elem(&response, "href").map(|elem| self.resource.combine(&elem.text()));
            let item_url = match item_url {
                None => {
                    log::warn!("Unable to extract HREF");
                    continue;
                }
                Some(resource) => resource.url().clone(),
            };

            let version_tag = match find_elem(&response, "getetag") {
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

    async fn get_item_by_url(&self, url: &Url) -> KFResult<Option<Item>> {
        let res = reqwest::Client::new()
            .get(url.clone())
            .header(CONTENT_TYPE, "text/calendar")
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .send()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url: url.clone(),
                method: Method::GET,
                source,
            })?;

        if !res.status().is_success() {
            return Err(KFError::UnexpectedHTTPStatusCode {
                expected: HttpStatusConstraint::Success,
                got: res.status(),
            });
        }

        let text = res
            .text()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url: url.clone(),
                method: Method::GET,
                source,
            })?;

        // This is supposed to be cached
        let version_tags = self.get_item_version_tags().await?;
        let vt = match version_tags.get(url) {
            None => return Err(RemoteCalendarError::ItemLacksVersionTag(url.clone()).into()),
            Some(vt) => vt,
        };

        let item = crate::ical::parse(&text, url.clone(), SyncStatus::Synced(vt.clone()))?;
        Ok(Some(item))
    }

    async fn get_items_by_url(&self, urls: &[Url]) -> KFResult<Vec<Option<Item>>> {
        // Build the request body
        let mut hrefs = String::new();
        for url in urls {
            hrefs.push_str(&format!("        <d:href>{}</d:href>\n", url.path()));
        }
        let body = format!("{}{}{}", MULTIGET_BODY_PREFIX, hrefs, MULTIGET_BODY_SUFFIX);

        // Send the request
        let xml_replies =
            sub_request_and_extract_elems(&self.resource, "REPORT", body, 1, "response").await?;

        // This is supposed to be cached
        let version_tags = self.get_item_version_tags().await?;

        // Parse the results
        let mut results = Vec::new();
        for xml_reply in xml_replies {
            let href = find_elem(&xml_reply, "href")
                .ok_or(KFError::MissingDOMElement {
                    text: xml_reply.text().clone(),
                    el: "href".into(),
                })?
                .text();
            let mut url = self.resource.url().clone();
            url.set_path(&href);
            let ical_data = find_elem(&xml_reply, "calendar-data")
                .ok_or(KFError::MissingDOMElement {
                    text: xml_reply.text().clone(),
                    el: "calendar-data".into(),
                })?
                .text();

            let vt = match version_tags.get(&url) {
                None => return Err(RemoteCalendarError::ItemLacksVersionTag(url.clone()).into()),
                Some(vt) => vt,
            };

            let item = crate::ical::parse(&ical_data, url.clone(), SyncStatus::Synced(vt.clone()))?;
            results.push(Some(item));
        }

        Ok(results)
    }

    async fn delete_item(&mut self, item_url: &Url) -> KFResult<()> {
        let del_response = reqwest::Client::new()
            .delete(item_url.clone())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .send()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url: item_url.clone(),
                method: Method::DELETE,
                source,
            })?;

        if !del_response.status().is_success() {
            return Err(KFError::UnexpectedHTTPStatusCode {
                expected: HttpStatusConstraint::Success,
                got: del_response.status(),
            });
        }

        Ok(())
    }

    async fn get_properties(&self) -> KFResult<Vec<Property>> {
        self.get_properties(&[PROP_ALLPROP.clone()]).await
    }

    async fn get_property(&self, nsn: &NamespacedName) -> KFResult<Option<Property>> {
        self.get_properties(&[nsn.clone()]).await.map(|props| {
            debug_assert_eq!(props.len(), 1);

            props.first().cloned()
        })
    }

    async fn delete_property(&mut self, nsn: &NamespacedName) -> KFResult<()> {
        let method: Method = "PROPPATCH".parse().expect("invalid method name");
        let url = self.url().clone();

        let propertyupdate = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
     <D:propertyupdate xmlns:D="DAV:" xmlns:A="{}">
       <D:remove>
         <D:prop><A:{}/></D:prop>
       </D:remove>
     </D:propertyupdate>"#,
            nsn.xmlns, nsn.name
        );

        let response = Box::pin(reqwest::Client::new())
            .request(method.clone(), url.clone())
            .header(CONTENT_TYPE, "application/xml")
            .header(CONTENT_LENGTH, propertyupdate.len())
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(propertyupdate)
            .send()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url,
                method,
                source,
            })?;

        if !response.status().is_success() {
            return Err(KFError::UnexpectedHTTPStatusCode {
                expected: HttpStatusConstraint::Success,
                got: response.status(),
            });
        }

        Ok(())
    }
}
