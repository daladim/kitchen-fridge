//! A module to parse ICal files

use std::error::Error;

use chrono::{DateTime, TimeZone, Utc};
use ical::parser::ical::component::{IcalCalendar, IcalEvent, IcalTodo};
use url::Url;

use crate::item::SyncStatus;
use crate::task::CompletionStatus;
use crate::Event;
use crate::Item;
use crate::Task;

/// Parse an iCal file into the internal representation [`crate::Item`]
pub fn parse(
    content: &str,
    item_url: Url,
    sync_status: SyncStatus,
) -> Result<Item, Box<dyn Error>> {
    let mut reader = ical::IcalParser::new(content.as_bytes());
    let parsed_item = match reader.next() {
        None => return Err(format!("Invalid iCal data to parse for item {}", item_url).into()),
        Some(item) => match item {
            Err(err) => {
                return Err(
                    format!("Unable to parse iCal data for item {}: {}", item_url, err).into(),
                )
            }
            Ok(item) => item,
        },
    };

    let ical_prod_id = extract_ical_prod_id(&parsed_item)
        .map(|s| s.to_string())
        .unwrap_or_else(|| super::default_prod_id());

    let item = match assert_single_type(&parsed_item)? {
        CurrentType::Event(_) => Item::Event(Event::new()),

        CurrentType::Todo(todo) => {
            let mut name = None;
            let mut uid = None;
            let mut completed = false;
            let mut last_modified = None;
            let mut completion_date = None;
            let mut creation_date = None;
            let mut extra_parameters = Vec::new();

            for prop in &todo.properties {
                match prop.name.as_str() {
                    "SUMMARY" => name = prop.value.clone(),
                    "UID" => uid = prop.value.clone(),
                    "DTSTAMP" => {
                        // The property can be specified once, but is not mandatory
                        // "This property specifies the date and time that the information associated with
                        //  the calendar component was last revised in the calendar store."
                        // "In the case of an iCalendar object that doesn't specify a "METHOD"
                        //  property [e.g.: VTODO and VEVENT], this property is equivalent to the "LAST-MODIFIED" property".
                        last_modified = parse_date_time_from_property(&prop.value);
                    }
                    "LAST-MODIFIED" => {
                        // The property can be specified once, but is not mandatory
                        // "This property specifies the date and time that the information associated with
                        //  the calendar component was last revised in the calendar store."
                        // In practise, for VEVENT and VTODO, this is generally the same value as DTSTAMP.
                        last_modified = parse_date_time_from_property(&prop.value);
                    }
                    "COMPLETED" => {
                        // The property can be specified once, but is not mandatory
                        // "This property defines the date and time that a to-do was
                        //  actually completed."
                        completion_date = parse_date_time_from_property(&prop.value)
                    }
                    "CREATED" => {
                        // The property can be specified once, but is not mandatory
                        creation_date = parse_date_time_from_property(&prop.value)
                    }
                    "STATUS" => {
                        // Possible values:
                        //   "NEEDS-ACTION" ;Indicates to-do needs action.
                        //   "COMPLETED"    ;Indicates to-do completed.
                        //   "IN-PROCESS"   ;Indicates to-do in process of.
                        //   "CANCELLED"    ;Indicates to-do was cancelled.
                        if prop.value.as_ref().map(|s| s.as_str()) == Some("COMPLETED") {
                            completed = true;
                        }
                    }
                    _ => {
                        // This field is not supported. Let's store it anyway, so that we are able to re-create an identical iCal file
                        extra_parameters.push(prop.clone());
                    }
                }
            }
            let name = match name {
                Some(name) => name,
                None => return Err(format!("Missing name for item {}", item_url).into()),
            };
            let uid = match uid {
                Some(uid) => uid,
                None => return Err(format!("Missing UID for item {}", item_url).into()),
            };
            let last_modified = match last_modified {
                Some(dt) => dt,
                None => {
                    return Err(format!(
                        "Missing DTSTAMP for item {}, but this is required by RFC5545",
                        item_url
                    )
                    .into())
                }
            };
            let completion_status = match completed {
                false => {
                    if completion_date.is_some() {
                        log::warn!("Task {:?} has an inconsistent content: its STATUS is not completed, yet it has a COMPLETED timestamp at {:?}", uid, completion_date);
                    }
                    CompletionStatus::Uncompleted
                }
                true => CompletionStatus::Completed(completion_date),
            };

            Item::Task(Task::new_with_parameters(
                name,
                uid,
                item_url,
                completion_status,
                sync_status,
                creation_date,
                last_modified,
                ical_prod_id,
                extra_parameters,
            ))
        }
    };

    // What to do with multiple items?
    if reader.next().map(|r| r.is_ok()) == Some(true) {
        return Err("Parsing multiple items are not supported".into());
    }

    Ok(item)
}

fn parse_date_time(dt: &str) -> Result<DateTime<Utc>, chrono::format::ParseError> {
    Utc.datetime_from_str(dt, "%Y%m%dT%H%M%SZ")
        .or_else(|_err| Utc.datetime_from_str(dt, "%Y%m%dT%H%M%S"))
}

fn parse_date_time_from_property(value: &Option<String>) -> Option<DateTime<Utc>> {
    value.as_ref().and_then(|s| {
        parse_date_time(s)
            .map_err(|err| {
                log::warn!("Invalid timestamp: {}", s);
                err
            })
            .ok()
    })
}

fn extract_ical_prod_id(item: &IcalCalendar) -> Option<&str> {
    for prop in &item.properties {
        if &prop.name == "PRODID" {
            return prop.value.as_ref().map(|s| s.as_str());
        }
    }
    None
}

enum CurrentType<'a> {
    Event(&'a IcalEvent),
    Todo(&'a IcalTodo),
}

fn assert_single_type<'a>(item: &'a IcalCalendar) -> Result<CurrentType<'a>, Box<dyn Error>> {
    let n_events = item.events.len();
    let n_todos = item.todos.len();
    let n_journals = item.journals.len();

    if n_events == 1 {
        if n_todos != 0 || n_journals != 0 {
            return Err("Only a single TODO or a single EVENT is supported".into());
        } else {
            return Ok(CurrentType::Event(&item.events[0]));
        }
    }

    if n_todos == 1 {
        if n_events != 0 || n_journals != 0 {
            return Err("Only a single TODO or a single EVENT is supported".into());
        } else {
            return Ok(CurrentType::Todo(&item.todos[0]));
        }
    }

    return Err("Only a single TODO or a single EVENT is supported".into());
}

#[cfg(test)]
mod test {
    const EXAMPLE_ICAL: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:0633de27-8c32-42be-bcb8-63bc879c6185@some-domain.com
CREATED:20210321T001600
LAST-MODIFIED:20210321T001600
DTSTAMP:20210321T001600
SUMMARY:Do not forget to do this
END:VTODO
END:VCALENDAR
"#;

    const EXAMPLE_ICAL_COMPLETED: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:19960401T080045Z-4000F192713-0052@example.com
CREATED:20210321T001600
LAST-MODIFIED:20210402T081557
DTSTAMP:20210402T081557
SUMMARY:Clean up your room or Mom will be angry
PERCENT-COMPLETE:100
COMPLETED:20210402T081557
STATUS:COMPLETED
END:VTODO
END:VCALENDAR
"#;

    const EXAMPLE_ICAL_COMPLETED_WITHOUT_A_COMPLETION_DATE: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:19960401T080045Z-4000F192713-0052@example.com
CREATED:20210321T001600
LAST-MODIFIED:20210402T081557
DTSTAMP:20210402T081557
SUMMARY:Clean up your room or Mom will be angry
STATUS:COMPLETED
END:VTODO
END:VCALENDAR
"#;

    const EXAMPLE_MULTIPLE_ICAL: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:0633de27-8c32-42be-bcb8-63bc879c6185
CREATED:20210321T001600
LAST-MODIFIED:20210321T001600
DTSTAMP:20210321T001600
SUMMARY:Call Mom
END:VTODO
END:VCALENDAR
BEGIN:VCALENDAR
BEGIN:VTODO
UID:0633de27-8c32-42be-bcb8-63bc879c6185
CREATED:20210321T001600
LAST-MODIFIED:20210321T001600
DTSTAMP:20210321T001600
SUMMARY:Buy a gift for Mom
END:VTODO
END:VCALENDAR
"#;

    use super::*;
    use crate::item::VersionTag;

    #[test]
    fn test_ical_parsing() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(EXAMPLE_ICAL, item_url.clone(), sync_status.clone()).unwrap();
        let task = item.unwrap_task();

        assert_eq!(task.name(), "Do not forget to do this");
        assert_eq!(task.url(), &item_url);
        assert_eq!(
            task.uid(),
            "0633de27-8c32-42be-bcb8-63bc879c6185@some-domain.com"
        );
        assert_eq!(task.completed(), false);
        assert_eq!(task.completion_status(), &CompletionStatus::Uncompleted);
        assert_eq!(task.sync_status(), &sync_status);
        assert_eq!(
            task.last_modified(),
            &Utc.ymd(2021, 03, 21).and_hms(0, 16, 0)
        );
    }

    #[test]
    fn test_completed_ical_parsing() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(
            EXAMPLE_ICAL_COMPLETED,
            item_url.clone(),
            sync_status.clone(),
        )
        .unwrap();
        let task = item.unwrap_task();

        assert_eq!(task.completed(), true);
        assert_eq!(
            task.completion_status(),
            &CompletionStatus::Completed(Some(Utc.ymd(2021, 04, 02).and_hms(8, 15, 57)))
        );
    }

    #[test]
    fn test_completed_without_date_ical_parsing() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(
            EXAMPLE_ICAL_COMPLETED_WITHOUT_A_COMPLETION_DATE,
            item_url.clone(),
            sync_status.clone(),
        )
        .unwrap();
        let task = item.unwrap_task();

        assert_eq!(task.completed(), true);
        assert_eq!(task.completion_status(), &CompletionStatus::Completed(None));
    }

    #[test]
    fn test_multiple_items_in_ical() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(EXAMPLE_MULTIPLE_ICAL, item_url.clone(), sync_status.clone());
        assert!(item.is_err());
    }
}
