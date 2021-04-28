//! Some tests of a CalDAV client.
//! Most of them are not really integration tests, but just development tests that should be cleaned up one day.

use reqwest::Method;
use reqwest::header::CONTENT_TYPE;
use minidom::Element;
use url::Url;

use kitchen_fridge::{calendar::SupportedComponents, client::Client};
use kitchen_fridge::traits::CalDavSource;

use kitchen_fridge::settings::URL;
use kitchen_fridge::settings::USERNAME;
use kitchen_fridge::settings::PASSWORD;
use kitchen_fridge::settings::EXAMPLE_TASK_URL;
use kitchen_fridge::settings::EXAMPLE_CREATED_CALENDAR_URL;


static EXAMPLE_TASKS_BODY_LAST_MODIFIED: &str = r#"
<C:calendar-query xmlns:D="DAV:"
              xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:getetag/>
    <C:calendar-data />
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VTODO">
        <C:prop-filter name="LAST-MODIFIED">
            <C:time-range start="20210228T002308Z"
                          end="20260105T000000Z"/>
        </C:prop-filter>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>
"#;

#[tokio::test]
#[ignore]
async fn show_calendars() {
    let _ = env_logger::builder().is_test(true).try_init();

    let client = Client::new(URL, USERNAME, PASSWORD).unwrap();
    let calendars = client.get_calendars().await.unwrap();

    println!("Calendars:");
    kitchen_fridge::utils::print_dav_calendar_list(&calendars).await;

}

#[tokio::test]
async fn create_cal() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut client = Client::new(URL, USERNAME, PASSWORD).unwrap();
    let id: Url = kitchen_fridge::settings::EXAMPLE_CREATED_CALENDAR_URL.parse().unwrap();
    let name = "prout".into();
    let supported_components = SupportedComponents::TODO;
    client.create_calendar(id, name, supported_components).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn profind() {
    let _ = env_logger::builder().is_test(true).try_init();

    let url: Url = EXAMPLE_TASK_URL.parse().unwrap();

    let method = Method::from_bytes(b"PROPFIND")
        .expect("cannot create PROPFIND method.");

    let res = reqwest::Client::new()
        .request(method, url.as_str())
        .header("Depth", 0)
        .header(CONTENT_TYPE, "application/xml")
        .basic_auth(USERNAME, Some(PASSWORD))
        //.body(body)
        .send()
        .await
        .unwrap();

    println!("{:?}", res.text().await);
}

#[tokio::test]
#[ignore]
async fn last_modified() {
    let _ = env_logger::builder().is_test(true).try_init();

    let url: Url = EXAMPLE_CREATED_CALENDAR_URL.parse().unwrap();

    let method = Method::from_bytes(b"REPORT")
        .expect("cannot create REPORT method.");

    let res = reqwest::Client::new()
        .request(method, url.as_str())
        .header("Depth", 1)
        .header(CONTENT_TYPE, "application/xml")
        .basic_auth(USERNAME, Some(PASSWORD))
        .body(EXAMPLE_TASKS_BODY_LAST_MODIFIED)
        .send()
        .await
        .unwrap();

    let el: Element = res.text().await.unwrap().parse().unwrap();
    kitchen_fridge::utils::print_xml(&el);
}




//
//
// TODO: test w/ wrong creds
// TODO: test withou connection
//
