//! This module provides a client to connect to a CalDAV server

use std::collections::HashMap;
use std::convert::TryFrom;
use std::error::Error;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use csscolorparser::Color;
use minidom::Element;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Method, StatusCode};
use url::Url;

use crate::calendar::remote_calendar::RemoteCalendar;
use crate::calendar::SupportedComponents;
use crate::resource::Resource;
use crate::traits::BaseCalendar;
use crate::traits::CalDavSource;
use crate::traits::DavCalendar;
use crate::utils::{find_elem, find_elems};

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

static CAL_BODY: &str = r#"
    <d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" >
       <d:prop>
         <d:displayname />
         <E:calendar-color xmlns:E="http://apple.com/ns/ical/"/>
         <d:resourcetype />
         <c:supported-calendar-component-set />
       </d:prop>
    </d:propfind>
"#;

pub(crate) async fn sub_request(
    resource: &Resource,
    method: &str,
    body: String,
    depth: u32,
) -> Result<String, Box<dyn Error>> {
    let method = method.parse().expect("invalid method name");

    let res = reqwest::Client::new()
        .request(method, resource.url().clone())
        .header("Depth", depth)
        .header(CONTENT_TYPE, "application/xml")
        .basic_auth(resource.username(), Some(resource.password()))
        .body(body)
        .send()
        .await?;

    if res.status().is_success() == false {
        return Err(format!("Unexpected HTTP status code {:?}", res.status()).into());
    }

    let text = res.text().await?;
    Ok(text)
}

pub(crate) async fn sub_request_and_extract_elem(
    resource: &Resource,
    body: String,
    items: &[&str],
) -> Result<String, Box<dyn Error>> {
    let text = sub_request(resource, "PROPFIND", body, 0).await?;

    let mut current_element: &Element = &text.parse()?;
    for item in items {
        current_element = match find_elem(&current_element, item) {
            Some(elem) => elem,
            None => return Err(format!("missing element {}", item).into()),
        }
    }
    Ok(current_element.text())
}

pub(crate) async fn sub_request_and_extract_elems(
    resource: &Resource,
    method: &str,
    body: String,
    item: &str,
) -> Result<Vec<Element>, Box<dyn Error>> {
    let text = sub_request(resource, method, body, 1).await?;

    let element: &Element = &text.parse()?;
    Ok(find_elems(&element, item)
        .iter()
        .map(|elem| (*elem).clone())
        .collect())
}

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
    ) -> Result<Self, Box<dyn Error>> {
        let url = Url::parse(url.as_ref())?;

        Ok(Self {
            resource: Resource::new(url, username.to_string(), password.to_string()),
            cached_replies: Mutex::new(CachedReplies::default()),
        })
    }

    /// Return the Principal URL, or fetch it from server if not known yet
    async fn get_principal(&self) -> Result<Resource, Box<dyn Error>> {
        if let Some(p) = &self.cached_replies.lock().unwrap().principal {
            return Ok(p.clone());
        }

        let href = sub_request_and_extract_elem(
            &self.resource,
            DAVCLIENT_BODY.into(),
            &["current-user-principal", "href"],
        )
        .await?;
        let principal_url = self.resource.combine(&href);
        self.cached_replies.lock().unwrap().principal = Some(principal_url.clone());
        log::debug!("Principal URL is {}", href);

        return Ok(principal_url);
    }

    /// Return the Homeset URL, or fetch it from server if not known yet
    async fn get_cal_home_set(&self) -> Result<Resource, Box<dyn Error>> {
        if let Some(h) = &self.cached_replies.lock().unwrap().calendar_home_set {
            return Ok(h.clone());
        }
        let principal_url = self.get_principal().await?;

        let href = sub_request_and_extract_elem(
            &principal_url,
            HOMESET_BODY.into(),
            &["calendar-home-set", "href"],
        )
        .await?;
        let chs_url = self.resource.combine(&href);
        self.cached_replies.lock().unwrap().calendar_home_set = Some(chs_url.clone());
        log::debug!("Calendar home set URL is {:?}", href);

        Ok(chs_url)
    }

    async fn populate_calendars(&self) -> Result<(), Box<dyn Error>> {
        let cal_home_set = self.get_cal_home_set().await?;

        let reps = sub_request_and_extract_elems(
            &cal_home_set,
            "PROPFIND",
            CAL_BODY.to_string(),
            "response",
        )
        .await?;
        let mut calendars = HashMap::new();
        for rep in reps {
            let display_name = find_elem(&rep, "displayname")
                .map(|e| e.text())
                .unwrap_or("<no name>".to_string());
            log::debug!("Considering calendar {}", display_name);

            // We filter out non-calendar items
            let resource_types = match find_elem(&rep, "resourcetype") {
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
            if found_calendar_type == false {
                continue;
            }

            // We filter out the root calendar collection, that has an empty supported-calendar-component-set
            let el_supported_comps = match find_elem(&rep, "supported-calendar-component-set") {
                None => continue,
                Some(comps) => comps,
            };
            if el_supported_comps.children().count() == 0 {
                continue;
            }

            let calendar_href = match find_elem(&rep, "href") {
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

            let this_calendar_color = find_elem(&rep, "calendar-color").and_then(|col| {
                col.texts()
                    .next()
                    .and_then(|t| csscolorparser::parse(t).ok())
            });

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

        let mut replies = self.cached_replies.lock().unwrap();
        replies.calendars = Some(calendars);
        Ok(())
    }
}

#[async_trait]
impl CalDavSource<RemoteCalendar> for Client {
    async fn get_calendars(
        &self,
    ) -> Result<HashMap<Url, Arc<Mutex<RemoteCalendar>>>, Box<dyn Error>> {
        self.populate_calendars().await?;

        match &self.cached_replies.lock().unwrap().calendars {
            Some(cals) => return Ok(cals.clone()),
            None => return Err("No calendars available".into()),
        };
    }

    async fn get_calendar(&self, url: &Url) -> Option<Arc<Mutex<RemoteCalendar>>> {
        if let Err(err) = self.populate_calendars().await {
            log::warn!("Unable to fetch calendars: {}", err);
            return None;
        }

        self.cached_replies
            .lock()
            .unwrap()
            .calendars
            .as_ref()
            .and_then(|cals| cals.get(url))
            .map(|cal| cal.clone())
    }

    async fn create_calendar(
        &mut self,
        url: Url,
        name: String,
        supported_components: SupportedComponents,
        color: Option<Color>,
    ) -> Result<Arc<Mutex<RemoteCalendar>>, Box<dyn Error>> {
        self.populate_calendars().await?;

        match self.cached_replies.lock().unwrap().calendars.as_ref() {
            None => return Err("No calendars have been fetched".into()),
            Some(cals) => {
                if cals.contains_key(&url) {
                    return Err("This calendar already exists".into());
                }
            }
        }

        let creation_body = calendar_body(name, supported_components, color);

        let response = reqwest::Client::new()
            .request(Method::from_bytes(b"MKCALENDAR").unwrap(), url.clone())
            .header(CONTENT_TYPE, "application/xml")
            .basic_auth(self.resource.username(), Some(self.resource.password()))
            .body(creation_body)
            .send()
            .await?;

        let status = response.status();
        if status != StatusCode::CREATED {
            return Err(format!(
                "Unexpected HTTP status code. Expected CREATED, got {}",
                status.as_u16()
            )
            .into());
        }

        self.get_calendar(&url)
            .await
            .ok_or(format!("Unable to insert calendar {:?}", url).into())
    }
}

fn calendar_body(
    name: String,
    supported_components: SupportedComponents,
    color: Option<Color>,
) -> String {
    let color_property = match color {
        None => "".to_string(),
        Some(color) => format!(
            "<D:calendar-color xmlns:D=\"http://apple.com/ns/ical/\">{}FF</D:calendar-color>",
            color.to_hex_string().to_ascii_uppercase()
        ),
    };

    // This is taken from https://tools.ietf.org/html/rfc4791#page-24
    format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
        <B:mkcalendar xmlns:B="urn:ietf:params:xml:ns:caldav">
            <A:set xmlns:A="DAV:">
                <A:prop>
                    <A:displayname>{}</A:displayname>
                    {}
                    {}
                </A:prop>
            </A:set>
        </B:mkcalendar>
        "#,
        name,
        color_property,
        supported_components.to_xml_string(),
    )
}
