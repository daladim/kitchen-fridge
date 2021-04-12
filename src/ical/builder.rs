//! A module to build ICal files

use std::error::Error;

use ics::properties::{Comment, Status, Summary};
use ics::{ICalendar, ToDo};

use crate::item::Item;
use crate::settings::{ORG_NAME, PRODUCT_NAME};

fn ical_product_id() -> String {
    format!("-//{}//{}//EN", ORG_NAME, PRODUCT_NAME)
}

/// Create an iCal item from a `crate::item::Item`
pub fn build_from(item: &Item) -> Result<String, Box<dyn Error>> {
    let mut todo = ToDo::new(item.uid(), "20181021T190000");
    todo.push(Summary::new("Take pictures of squirrels (with ÜTF-8 chars)"));
    todo.push(Comment::new("That's really something I'd like to do one day"));

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



#[cfg(test)]
mod tests {
    use super::*;
    use crate::Task;

    #[test]
    fn test_ical_from_task() {
        let cal_id = "http://my.calend.ar/id".parse().unwrap();
        let task = Item::Task(Task::new(
            String::from("This is a task"), true, &cal_id
        ));
        let expected_ical = format!("BEGIN:VCALENDAR\r\n\
            VERSION:2.0\r\n\
            PRODID:-//{}//{}//EN\r\n\
            BEGIN:VTODO\r\n\
            UID:{}\r\n\
            DTSTAMP:20181021T190000\r\n\
            SUMMARY:Take pictures of squirrels (with ÜTF-8 chars)\r\n\
            COMMENT:That's really something I'd like to do one day\r\n\
            STATUS:COMPLETED\r\n\
            END:VTODO\r\n\
            END:VCALENDAR\r\n", ORG_NAME, PRODUCT_NAME, task.uid());

        let ical = build_from(&task);
        assert_eq!(ical.unwrap(), expected_ical);
    }

    #[test]
    #[ignore]
    fn test_ical_from_event() {
        unimplemented!();
    }
}
