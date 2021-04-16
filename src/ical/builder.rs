//! A module to build ICal files

use std::error::Error;

use chrono::{DateTime, Utc};
use ics::properties::{Completed, Created, LastModified, PercentComplete, Status, Summary};
use ics::{ICalendar, ToDo};

use crate::item::Item;
use crate::task::CompletionStatus;
use crate::settings::{ORG_NAME, PRODUCT_NAME};

fn ical_product_id() -> String {
    format!("-//{}//{}//EN", ORG_NAME, PRODUCT_NAME)
}

/// Create an iCal item from a `crate::item::Item`
pub fn build_from(item: &Item) -> Result<String, Box<dyn Error>> {
    let s_last_modified = format_date_time(item.last_modified());

    let mut todo = ToDo::new(
        item.uid(),
        s_last_modified.clone(),
    );

    item.creation_date().map(|dt|
        todo.push(Created::new(format_date_time(dt)))
    );
    todo.push(LastModified::new(s_last_modified));
    todo.push(Summary::new(item.name()));

    match item {
        Item::Task(t) => {
            match t.completion_status() {
                CompletionStatus::Uncompleted => {
                    todo.push(Status::needs_action());
                },
                CompletionStatus::Completed(completion_date) => {
                    todo.push(PercentComplete::new("100"));
                    completion_date.as_ref().map(|dt| todo.push(
                        Completed::new(format_date_time(dt))
                    ));
                    todo.push(Status::completed());
                }
            }
        },
        _ => {
            unimplemented!()
        },
    }

    let mut calendar = ICalendar::new("2.0", ical_product_id());
    calendar.add_todo(todo);

    Ok(calendar.to_string())
}

fn format_date_time(dt: &DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%S").to_string()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::Task;

    #[test]
    fn test_ical_from_completed_task() {
        let (s_now, uid, ical) = build_task(true);

        let expected_ical = format!("BEGIN:VCALENDAR\r\n\
            VERSION:2.0\r\n\
            PRODID:-//{}//{}//EN\r\n\
            BEGIN:VTODO\r\n\
            UID:{}\r\n\
            DTSTAMP:{}\r\n\
            CREATED:{}\r\n\
            LAST-MODIFIED:{}\r\n\
            SUMMARY:This is a task with ÜTF-8 characters\r\n\
            PERCENT-COMPLETE:100\r\n\
            COMPLETED:{}\r\n\
            STATUS:COMPLETED\r\n\
            END:VTODO\r\n\
            END:VCALENDAR\r\n", ORG_NAME, PRODUCT_NAME, uid, s_now, s_now, s_now, s_now);

        assert_eq!(ical, expected_ical);
    }

    #[test]
    fn test_ical_from_uncompleted_task() {
        let (s_now, uid, ical) = build_task(false);

        let expected_ical = format!("BEGIN:VCALENDAR\r\n\
            VERSION:2.0\r\n\
            PRODID:-//{}//{}//EN\r\n\
            BEGIN:VTODO\r\n\
            UID:{}\r\n\
            DTSTAMP:{}\r\n\
            CREATED:{}\r\n\
            LAST-MODIFIED:{}\r\n\
            SUMMARY:This is a task with ÜTF-8 characters\r\n\
            STATUS:NEEDS-ACTION\r\n\
            END:VTODO\r\n\
            END:VCALENDAR\r\n", ORG_NAME, PRODUCT_NAME, uid, s_now, s_now, s_now);

        assert_eq!(ical, expected_ical);
    }

    fn build_task(completed: bool) -> (String, String, String) {
        let cal_id = "http://my.calend.ar/id".parse().unwrap();
        let now = Utc::now();
        let s_now = format_date_time(&now);

        let task = Item::Task(Task::new(
            String::from("This is a task with ÜTF-8 characters"), completed, &cal_id
        ));

        let ical = build_from(&task).unwrap();
        (s_now, task.uid().to_string(), ical)
    }

    #[test]
    #[ignore]
    fn test_ical_from_event() {
        unimplemented!();
    }
}
