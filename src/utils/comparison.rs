//! Utilities to compare custom types
//!
//! These can be used to sort results, e.g. by using `sorted_by` from the `itertools` crate

use crate::item::{Item, ItemId};

/// Compare alphabetically types returned e.g. by [`crate::traits::CompleteCalendar::get_items`]
pub fn compare_items_alpha(left: &(&ItemId, &&Item), right: &(&ItemId, &&Item)) -> std::cmp::Ordering {
    Ord::cmp(&left.1.name().to_lowercase(), &right.1.name().to_lowercase())
}
