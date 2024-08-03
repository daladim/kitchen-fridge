//! This module handles conversion between iCal files and internal representations
//!
//! It is a wrapper around different Rust third-party libraries, since I haven't find any complete library that is able to parse _and_ generate iCal files

mod parser;
pub use parser::parse;
mod builder;
pub use builder::build_from;

use crate::config::{ORG_NAME, PRODUCT_NAME};

pub fn default_prod_id() -> String {
    format!(
        "-//{}//{}//EN",
        ORG_NAME.lock().unwrap(),
        PRODUCT_NAME.lock().unwrap()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::item::SyncStatus;
    use std::collections::HashSet;

    #[test]
    fn test_ical_round_trip_serde() {
        let ical_with_unknown_fields =
            std::fs::read_to_string("tests/assets/ical_with_unknown_fields.ics").unwrap();

        let item_id = "http://item.id".parse().unwrap();
        let sync_status = SyncStatus::NotSynced;
        let deserialized = parse(&ical_with_unknown_fields, item_id, sync_status).unwrap();
        let serialized = build_from(&deserialized).unwrap();
        assert_same_fields(&ical_with_unknown_fields, &serialized);
    }

    /// Assert the properties are present (possibly in another order)
    /// RFC5545 "imposes no ordering of properties within an iCalendar object."
    fn assert_same_fields(left: &str, right: &str) {
        let left_parts: HashSet<&str> = left.split("\r\n").collect();
        let right_parts: HashSet<&str> = right.split("\r\n").collect();

        // Let's be more explicit than assert_eq!(left_parts, right_parts);
        if left_parts != right_parts {
            println!("Only in left:");
            for item in left_parts.difference(&right_parts) {
                println!("  * {}", item);
            }
            println!("Only in right:");
            for item in right_parts.difference(&left_parts) {
                println!("  * {}", item);
            }

            assert_eq!(left_parts, right_parts);
        }
    }
}
