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

pub struct Client {
    url: Url,
    username: String,
    password: String,

    principal: Option<Url>,
    calendar_home_set: Option<Url>,
}

impl Client {
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

    /// Return the Principal URL, or fetch it from server if not known yet
    async fn get_principal(&mut self) -> Result<Url, Box<dyn Error>> {
        if let Some(p) = &self.principal {
            return Ok(p.clone());
        }

        let method = Method::from_bytes(b"PROPFIND")
            .expect("cannot create PROPFIND method.");

        let res = reqwest::Client::new()
            .request(method, self.url.as_str())
            .header("Depth", 0)
            .header(CONTENT_TYPE, "application/xml")
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .body(DAVCLIENT_BODY)
            .send()
            .await?;
        let text = res.text().await?;

        let root: Element = text.parse().unwrap();
        let principal = find_elem(&root, "current-user-principal".to_string()).unwrap();
        let principal_href = find_elem(principal, "href".to_string()).unwrap();
        let h_str = principal_href.text();

        eprintln!("URL is {}", h_str);

        let mut principal_url = self.url.clone();
        principal_url.set_path(&h_str);
        self.principal = Some(principal_url.clone());

        return Ok(principal_url);
    }

    /// Return the Homeset URL, or fetch it from server if not known yet
    async fn get_cal_home_set(&mut self) -> Result<Url, Box<dyn Error>> {
        if let Some(h) = &self.calendar_home_set {
            return Ok(h.clone());
        }
        let principal_url = self.get_principal().await?;

        let method = Method::from_bytes(b"PROPFIND")
            .expect("cannot create PROPFIND method. principal");

        let res = reqwest::Client::new()
            .request(method, principal_url.as_str())
            .header("Depth", 0)
            .header(CONTENT_TYPE, "application/xml")
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .body(HOMESET_BODY)
            .send()
            .await?;

        let text = res.text().await?;

        let root: Element = text.parse().unwrap();
        let chs = find_elem(&root, "calendar-home-set".to_string()).unwrap();
        let chs_href = find_elem(chs, "href".to_string()).unwrap();
        let chs_str = chs_href.text();

        let mut chs_url = self.url.clone();
        chs_url.set_path(&chs_str);
        println!("Calendar home set {:?}", chs_url.path());
        self.calendar_home_set = Some(chs_url.clone());

        Ok(chs_url)
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
        client.get_cal_home_set().await.unwrap();
    }
}
