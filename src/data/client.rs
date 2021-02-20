//! Code to connect to a Caldav server
//!
//! Some of it comes from https://github.com/marshalshi/caldav-client-rust.git

use std::error::Error;
use std::convert::TryFrom;

use reqwest::Method;
use reqwest::header::CONTENT_TYPE;
use minidom::Element;
use url::Url;

use crate::data::Calendar;

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
         <d:resourcetype />
         <c:supported-calendar-component-set />
       </d:prop>
    </d:propfind>
"#;

pub struct Client {
    url: Url,
    username: String,
    password: String,

    principal: Option<Url>,
    calendar_home_set: Option<Url>,
    calendars: Option<Vec<Calendar>>,
}

impl Client {
    /// Create a client. This does not start a connection
    pub fn new<S: AsRef<str>, T: ToString, U: ToString>(url: S, username: T, password: U) -> Result<Self, Box<dyn Error>> {
        let url = Url::parse(url.as_ref())?;

        Ok(Self{
            url,
            username: username.to_string(),
            password: password.to_string(),
            principal: None,
            calendar_home_set: None,
            calendars: None,
        })
    }

    async fn sub_request(&self, url: &Url, body: String, depth: u32) -> Result<String, Box<dyn Error>> {
        let method = Method::from_bytes(b"PROPFIND")
            .expect("cannot create PROPFIND method.");

        let res = reqwest::Client::new()
            .request(method, url.as_str())
            .header("Depth", depth)
            .header(CONTENT_TYPE, "application/xml")
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .body(body)
            .send()
            .await?;
        let text = res.text().await?;
        Ok(text)
    }

    async fn sub_request_and_process(&self, url: &Url, body: String, items: &[&str]) -> Result<String, Box<dyn Error>> {
        let text = self.sub_request(url, body, 0).await?;

        let mut current_element: &Element = &text.parse().unwrap();
        items.iter()
            .map(|item| {
                current_element = find_elem(&current_element, item.to_string()).unwrap();
            })
            .collect::<()>();

        Ok(current_element.text())
    }

    /// Return the Principal URL, or fetch it from server if not known yet
    async fn get_principal(&mut self) -> Result<Url, Box<dyn Error>> {
        if let Some(p) = &self.principal {
            return Ok(p.clone());
        }

        let href = self.sub_request_and_process(&self.url, DAVCLIENT_BODY.into(), &["current-user-principal", "href"]).await?;
        let mut principal_url = self.url.clone();
        principal_url.set_path(&href);
        self.principal = Some(principal_url.clone());
        log::debug!("Principal URL is {}", href);

        return Ok(principal_url);
    }

    /// Return the Homeset URL, or fetch it from server if not known yet
    async fn get_cal_home_set(&mut self) -> Result<Url, Box<dyn Error>> {
        if let Some(h) = &self.calendar_home_set {
            return Ok(h.clone());
        }
        let principal_url = self.get_principal().await?;

        let href = self.sub_request_and_process(&principal_url, HOMESET_BODY.into(), &["calendar-home-set", "href"]).await?;
        let mut chs_url = self.url.clone();
        chs_url.set_path(&href);
        self.calendar_home_set = Some(chs_url.clone());
        log::debug!("Calendar home set URL is {:?}", chs_url.path());

        Ok(chs_url)
    }

    /// Return the list of calendars, or fetch from server if not known yet
    pub async fn get_calendars(&mut self) -> Result<Vec<Calendar>, Box<dyn Error>> {
        if let Some(c) = &self.calendars {
            return Ok(c.to_vec());
        }
        let cal_home_set = self.get_cal_home_set().await?;

        let text = self.sub_request(&cal_home_set, CAL_BODY.into(), 1).await?;

        let root: Element = text.parse().unwrap();
        let reps = find_elems(&root, "response".to_string());
        let mut calendars = Vec::new();
        for rep in reps {
            let display_name = find_elem(rep, "displayname".to_string()).map(|e| e.text()).unwrap_or("<no name>".to_string());
            log::debug!("Considering calendar {}", display_name);

            // We filter out non-calendar items
            let resource_types = match find_elem(rep, "resourcetype".to_string()) {
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
            let el_supported_comps = match find_elem(rep, "supported-calendar-component-set".to_string()) {
                None => continue,
                Some(comps) => comps,
            };
            if el_supported_comps.children().count() == 0 {
                continue;
            }

            let calendar_href = match find_elem(rep, "href".to_string()) {
                None => {
                    log::warn!("Calendar {} has no URL! Ignoring it.", display_name);
                    continue;
                },
                Some(h) => h.text(),
            };

            let mut this_calendar_url = self.url.clone();
            this_calendar_url.set_path(&calendar_href);

            let supported_components = match crate::data::calendar::SupportedComponents::try_from(el_supported_comps.clone()) {
                Err(err) => {
                    log::warn!("Calendar {} has invalid supported components ({})! Ignoring it.", display_name, err);
                    continue;
                },
                Ok(sc) => sc,
            };
            let this_calendar = Calendar::new(display_name, this_calendar_url, supported_components);
            log::info!("Found calendar {}", this_calendar.name());
            calendars.push(this_calendar);
        }

        self.calendars = Some(calendars.clone());
        Ok(calendars)
    }
}


/// Walks the tree and returns every element that has the given name
pub fn find_elems(root: &Element, searched_name: String) -> Vec<&Element> {
    let mut elems: Vec<&Element> = Vec::new();

    for el in root.children() {
        if el.name() == searched_name {
            elems.push(el);
        } else {
            let ret = find_elems(el, searched_name.clone());
            elems.extend(ret);
        }
    }
    elems
}

/// Walks the tree until it finds an elements with the given name
pub fn find_elem(root: &Element, searched_name: String) -> Option<&Element> {
    if root.name() == searched_name {
        return Some(root);
    }

    for el in root.children() {
        if el.name() == searched_name {
            return Some(el);
        } else {
            let ret = find_elem(el, searched_name.clone());
            if ret.is_some() {
                return ret;
            }
        }
    }
    None
}



#[cfg(test)]
mod test {
    use super::*;

    use crate::settings::URL;
    use crate::settings::USERNAME;
    use crate::settings::PASSWORD;

    #[tokio::test]
    async fn test_client() {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut client = Client::new(URL, USERNAME, PASSWORD).unwrap();
        let calendars = client.get_calendars().await.unwrap();

        println!("Calendars:");
        calendars.iter()
            .map(|cal| println!("  {}", cal.name()))
            .collect::<()>();
    }
}
