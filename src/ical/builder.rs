//! A module to build ICal files

use std::error::Error;

use chrono::{DateTime, Utc};
use ics::properties::{LastModified, Status, Summary};
use ics::{ICalendar, ToDo};

use crate::item::Item;
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
    todo.push(LastModified::new(s_last_modified));
    todo.push(Summary::new(item.name()));

    match item {
        Item::Task(t) => {
            let status = if t.completed() { Status::completed() } else { Status::needs_action() };
            todo.push(status);
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
    fn test_ical_from_task() {
        let cal_id = "http://my.calend.ar/id".parse().unwrap();
        let now = format_date_time(&Utc::now());

        let task = Item::Task(Task::new(
            String::from("This is a task with ÜTF-8 characters"), true, &cal_id
        ));
        let expected_ical = format!("BEGIN:VCALENDAR\r\n\
            VERSION:2.0\r\n\
            PRODID:-//{}//{}//EN\r\n\
            BEGIN:VTODO\r\n\
            UID:{}\r\n\
            DTSTAMP:{}\r\n\
            SUMMARY:This is a task with ÜTF-8 characters\r\n\
            STATUS:COMPLETED\r\n\
            END:VTODO\r\n\
            END:VCALENDAR\r\n", ORG_NAME, PRODUCT_NAME, task.uid(), now);

        let ical = build_from(&task);
        assert_eq!(ical.unwrap(), expected_ical);
    }

    #[test]
    #[ignore]
    fn test_ical_from_event() {
        unimplemented!();
    }
}
