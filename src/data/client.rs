//! Code to connect to a Caldav server
//!
//! Some of it comes from https://github.com/marshalshi/caldav-client-rust.git

use std::error::Error;

use reqwest::Method;
use reqwest::header::CONTENT_TYPE;
use minidom::Element;
use url::Url;

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
        println!("URL is {}", href);

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
        println!("Calendar home set {:?}", chs_url.path());

        Ok(chs_url)
    }

    pub async fn get_calendars(&mut self) -> Result<(), Box<dyn Error>> {
        let cal_home_set = self.get_cal_home_set().await?;

        let text = self.sub_request(&cal_home_set, CAL_BODY.into(), 1).await?;
        println!("TEXT {}", text);
        let root: Element = text.parse().unwrap();
        let reps = find_elems(&root, "response".to_string());
        for rep in reps {
            // TODO checking `displayname` here but may there are better way
            let displayname = find_elem(rep, "displayname".to_string())
                .unwrap()
                .text();
            if displayname == "" {
                continue;
            }

            // TODO: filter by:
            // <cal:supported-calendar-component-set>
            //     <cal:comp name=\"VEVENT\"/>
            //     <cal:comp name=\"VTODO\"/>
            // </cal:supported-calendar-component-set>

            let href = find_elem(rep, "href".to_string()).unwrap();
            let href_text = href.text();
            println!("href: {:?}", href_text);
            //self.calendars.push(href_text.to_string());
        }

        Ok(())

    }
}



pub fn find_elems(root: &Element, tag: String) -> Vec<&Element> {
    let mut elems: Vec<&Element> = Vec::new();

    for el in root.children() {
        if el.name() == tag {
            elems.push(el);
        } else {
            let ret = find_elems(el, tag.clone());
            elems.extend(ret);
        }
    }
    elems
}

pub fn find_elem(root: &Element, tag: String) -> Option<&Element> {
    if root.name() == tag {
        return Some(root);
    }

    for el in root.children() {
        if el.name() == tag {
            return Some(el);
        } else {
            let ret = find_elem(el, tag.clone());
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
        let mut client = Client::new(URL, USERNAME, PASSWORD).unwrap();
        client.get_calendars().await.unwrap();
    }
}
