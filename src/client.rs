//! This module provides a client to connect to a CalDAV server

use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;

use async_trait::async_trait;
use csscolorparser::Color;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Method, StatusCode};
use tokio::sync::Mutex;
use url::Url;

use crate::calendar::remote_calendar::RemoteCalendar;
use crate::calendar::SupportedComponents;
use crate::error::{HttpStatusConstraint, KFError, KFResult};
use crate::item::ItemType;
use crate::resource::Resource;
use crate::traits::BaseCalendar;
use crate::traits::CalDavSource;
use crate::traits::DavCalendar;
use crate::utils::prop::{
    Property, PROP_CALENDAR_COLOR, PROP_DISPLAY_NAME, PROP_RESOURCE_TYPE,
    PROP_SUPPORTED_CALENDAR_COMPONENT_SET,
};
use crate::utils::req::{
    propfind_body, sub_request_and_extract_elem, sub_request_and_extract_elems,
};
use crate::utils::xml::find_elem;
use crate::utils::Namespaces;

static DAVCLIENT_BODY: &str = r#"
    <d:propfind xmlns:d="DAV:">
       <d:prop>
           <d:current-user-principal />
       </d:prop>
    </d:propfind>
"#;

static HOMESET_BODY: &str = r#"
    <d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
      <d:self/>
      <d:prop>
        <c:calendar-home-set />
      </d:prop>
    </d:propfind>
"#;

/// A CalDAV data source that fetches its data from a CalDAV server
#[derive(Debug)]
pub struct Client {
    resource: Resource,

    /// The interior mutable part of a Client.
    /// This data may be retrieved once and then cached
    cached_replies: Mutex<CachedReplies>,
}

#[derive(Debug, Default)]
struct CachedReplies {
    principal: Option<Resource>,
    calendar_home_set: Option<Resource>,
    calendars: Option<HashMap<Url, Arc<Mutex<RemoteCalendar>>>>,
}

impl Client {
    /// Create a client. This does not start a connection
    pub fn new<S: AsRef<str>, T: ToString, U: ToString>(
        url: S,
        username: T,
        password: U,
    ) -> Result<Self, url::ParseError> {
        let url = Url::parse(url.as_ref())?;

        Ok(Self {
            resource: Resource::new(url, username.to_string(), password.to_string()),
            cached_replies: Mutex::new(CachedReplies::default()),
        })
    }

    /// Return the Principal URL, or fetch it from server if not known yet
    async fn get_principal(&self) -> KFResult<Resource> {
        if let Some(p) = &self.cached_replies.lock().await.principal {
            return Ok(p.clone());
        }

        let href = sub_request_and_extract_elem(
            &self.resource,
            DAVCLIENT_BODY.into(),
            0,
            &["current-user-principal", "href"],
        )
        .await?;
        let principal_url = self.resource.combine(&href);
        self.cached_replies.lock().await.principal = Some(principal_url.clone());
        log::debug!("Principal URL is {}", href);

        Ok(principal_url)
    }

    /// Return the Homeset URL, or fetch it from server if not known yet
    pub async fn get_cal_home_set(&self) -> KFResult<Resource> {
        if let Some(h) = &self.cached_replies.lock().await.calendar_home_set {
            return Ok(h.clone());
        }
        let principal_url = self.get_principal().await?;

        let href = sub_request_and_extract_elem(
            &principal_url,
            HOMESET_BODY.into(),
            0,
            &["calendar-home-set", "href"],
        )
        .await?;
        let chs_url = self.resource.combine(&href);
        self.cached_replies.lock().await.calendar_home_set = Some(chs_url.clone());
        log::debug!("Calendar home set URL is {:?}", href);

        Ok(chs_url)
    }

    /// Based on a PROPFIND call, discovers accessible calendars on the server and instantiates RemoteCalendar's to
    /// represent them.
    async fn populate_calendars(&self) -> KFResult<()> {
        let cal_home_set = self.get_cal_home_set().await?;
        let props = &[
            PROP_CALENDAR_COLOR.clone(),
            PROP_DISPLAY_NAME.clone(),
            PROP_RESOURCE_TYPE.clone(),
            PROP_SUPPORTED_CALENDAR_COMPONENT_SET.clone(),
        ];
        let body = propfind_body(props);

        let responses =
            sub_request_and_extract_elems(&cal_home_set, "PROPFIND", body, 1, "response").await?;
        let mut calendars = HashMap::new();
        for response in responses {
            let display_name = find_elem(&response, "displayname")
                .map(|e| e.text())
                .unwrap_or("<no name>".to_string());
            log::debug!("Considering calendar {}", display_name);

            // We filter out non-calendar items
            let resource_types = match find_elem(&response, "resourcetype") {
                None => continue,
                Some(rt) => rt,
            };
            let mut found_calendar_type = false;
            for resource_type in resource_types.children() {
                if resource_type.name() == "calendar" {
                    found_calendar_type = true;
                    break;
                }
            }
            if !found_calendar_type {
                continue;
            }

            // We filter out the root calendar collection, that has an empty supported-calendar-component-set
            let el_supported_comps = match find_elem(&response, "supported-calendar-component-set")
            {
                None => continue,
                Some(comps) => comps,
            };
            if el_supported_comps.children().count() == 0 {
                continue;
            }

            let calendar_href = match find_elem(&response, "href") {
                None => {
                    log::warn!("Calendar {} has no URL! Ignoring it.", display_name);
                    continue;
                }
                Some(h) => h.text(),
            };

            let this_calendar_url = self.resource.combine(&calendar_href);

            let supported_components =
                match crate::calendar::SupportedComponents::try_from(el_supported_comps.clone()) {
                    Err(err) => {
                        log::warn!(
                            "Calendar {} has invalid supported components ({})! Ignoring it.",
                            display_name,
                            err
                        );
                        continue;
                    }
                    Ok(sc) => sc,
                };

            let this_calendar_color = find_elem(&response, "calendar-color").and_then(|col| {
                col.texts()
                    .next()
                    .and_then(|t| csscolorparser::parse(t).ok())
            });

            // let all_properties = {
            //     let mut all = Vec::new();
            //     let propstat = find_elem(&response, "propstat").unwrap();
            //     let prop = find_elem(&propstat, "prop").unwrap();
            //     for prop_el in prop.children() {
            //         let ns = prop_el.ns();
            //         let name = prop_el.name();
            //         let value = prop_el.text();

            //         all.push(Property::new(ns, name, value));
            //     }

            //     all
            // };

            let this_calendar = RemoteCalendar::new(
                display_name,
                this_calendar_url,
                supported_components,
                this_calendar_color,
            );
            log::info!("Found calendar {}", this_calendar.name());
            calendars.insert(
                this_calendar.url().clone(),
                Arc::new(Mutex::new(this_calendar)),
            );
        }

        let mut replies = self.cached_replies.lock().await;
        replies.calendars = Some(calendars);
        Ok(())
    }
}

#[async_trait]
impl CalDavSource<RemoteCalendar> for Client {
    async fn get_calendars(&self) -> KFResult<HashMap<Url, Arc<Mutex<RemoteCalendar>>>> {
        self.populate_calendars().await?;

        Ok(self
            .cached_replies
            .lock()
            .await
            .calendars
            .as_ref()
            .unwrap() // Unwrap OK because populate_calendars either does what it says, or returns Err
            .clone())
    }

    async fn get_calendar(&self, url: &Url) -> Option<Arc<Mutex<RemoteCalendar>>> {
        if let Err(err) = self.populate_calendars().await {
            log::warn!("Unable to fetch calendars: {}", err);
            return None;
        }

        self.cached_replies
            .lock()
            .await
            .calendars
            .as_ref()
            .and_then(|cals| cals.get(url))
            .cloned()
    }

    /// Makes a MKCALENDAR call to create a calendar on the server.
    async fn create_calendar(
        &mut self,
        url: Url,
        name: String,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> KFResult<Arc<Mutex<RemoteCalendar>>> {
        self.populate_calendars().await?;

        let cals = self
            .cached_replies
            .lock()
            .await
            .calendars
            .as_ref()
            .unwrap()
            .clone();

        if cals.contains_key(&url) {
            return Err(KFError::ItemAlreadyExists {
                type_: ItemType::Calendar,
                detail: "".into(),
                url,
            });
        }

        //NOTE This does not make use of `calendar_body`'s ability to define calendar properties in the MKCALENDAR call
        let creation_body = calendar_body(name, supported_components, color, Default::default());

        let method = Method::from_bytes(b"MKCALENDAR").unwrap();

        let response = reqwest::Client::new()
            .request(method.clone(), url.clone())
            .header(CONTENT_TYPE, "application/xml")
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(creation_body)
            .send()
            .await
            .map_err(|e| KFError::HttpRequestError {
                method,
                url: url.clone(),
                source: e,
            })?;

        let status = response.status();
        if status != StatusCode::CREATED {
            return Err(KFError::UnexpectedHTTPStatusCode {
                expected: HttpStatusConstraint::Specific(vec![StatusCode::CREATED]),
                got: status,
            });
        }

        self.get_calendar(&url)
            .await
            .ok_or(KFError::CalendarDidNotSyncAfterCreation(url))
    }

    async fn delete_calendar(&mut self, url: &Url) -> KFResult<Option<Arc<Mutex<RemoteCalendar>>>> {
        // First, attempt to delete the calendar on the remote server:
        let response = reqwest::Client::new()
            .request(Method::DELETE, url.clone())
            .header(CONTENT_TYPE, "application/xml")
            .basic_auth(
                self.resource.username().to_string(),
                Some(self.resource.password().to_string()),
            )
            .send()
            .await
            .map_err(|source| KFError::HttpRequestError {
                url: url.clone(),
                method: Method::DELETE,
                source,
            })?;

        // Check that some acceptable HTTP status was returned
        // In WebDAV, a 207 Multistatus status on DELETE implies that the entire deletion failed, since it's all or nothing
        let status = response.status();

        let constraint =
            HttpStatusConstraint::Specific(vec![StatusCode::OK, StatusCode::NO_CONTENT]);

        constraint
            .assert(status)
            .map_err(|_| KFError::ItemDoesNotExist {
                detail: "Can't delete calendar".into(),
                url: url.clone(),
                type_: Some(ItemType::Calendar),
            })?;

        // Now that we've removed the calendar from the server, evict it from the cached replies (if present)
        let mut replies = self.cached_replies.lock().await;
        let cals = replies.calendars.as_mut();
        Ok(cals.unwrap().remove(url))
    }
}

fn calendar_body(
    name: String,
    supported_components: SupportedComponents,
    color: Option<Color>,
    properties: Vec<Property>,
) -> String {
    let color_property = match color {
        None => "".to_string(),
        Some(color) => format!(
            "<D:calendar-color xmlns:D=\"http://apple.com/ns/ical/\">{}FF</D:calendar-color>",
            color.to_hex_string().to_ascii_uppercase()
        ),
    };

    let mut namespaces = Namespaces::new();

    for p in &properties {
        namespaces.add(p.xmlns());
    }

    let other_props: String = {
        let mut s = String::new();
        for p in properties {
            // <{}:{}>{}</{}:{}>\n

            let symbolized = p.nsn().with_symbolized_prefix(&namespaces);
            s.push('<');
            s.push_str(symbolized.as_str());
            s.push('>');
            s.push_str(p.value().as_str());
            s.push('<');
            s.push('/');
            s.push_str(symbolized.as_str());
            s.push('>');
            s.push('\n');
        }
        s
    };

    // This is taken from https://tools.ietf.org/html/rfc4791#page-24
    format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
        <B:mkcalendar xmlns:B="urn:ietf:params:xml:ns:caldav">
            <A:set{}>
                <A:prop>
                    <A:displayname>{}</A:displayname>
                    {}
                    {}
                    {}
                </A:prop>
            </A:set>
        </B:mkcalendar>
        "#,
        namespaces.decl(),
        name,
        color_property,
        supported_components.to_xml_string(),
        other_props
    )
}
