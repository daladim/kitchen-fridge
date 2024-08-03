//! Some utility functions

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::{stdin, stdout, Read, Write};
use std::sync::{Arc, Mutex};

use minidom::Element;
use url::Url;

use crate::item::SyncStatus;
use crate::traits::CompleteCalendar;
use crate::traits::DavCalendar;
use crate::Item;

/// Walks an XML tree and returns every element that has the given name
pub fn find_elems<S: AsRef<str>>(root: &Element, searched_name: S) -> Vec<&Element> {
    let searched_name = searched_name.as_ref();
    let mut elems: Vec<&Element> = Vec::new();

    for el in root.children() {
        if el.name() == searched_name {
            elems.push(el);
        } else {
            let ret = find_elems(el, searched_name);
            elems.extend(ret);
        }
    }
    elems
}

/// Walks an XML tree until it finds an elements with the given name
pub fn find_elem<S: AsRef<str>>(root: &Element, searched_name: S) -> Option<&Element> {
    let searched_name = searched_name.as_ref();
    if root.name() == searched_name {
        return Some(root);
    }

    for el in root.children() {
        if el.name() == searched_name {
            return Some(el);
        } else {
            let ret = find_elem(el, searched_name);
            if ret.is_some() {
                return ret;
            }
        }
    }
    None
}

pub fn print_xml(element: &Element) {
    let mut writer = std::io::stdout();

    let mut xml_writer = minidom::quick_xml::Writer::new_with_indent(std::io::stdout(), 0x20, 4);
    let _ = element.to_writer(&mut xml_writer);
    let _ = writer.write(&[0x0a]);
}

/// A debug utility that pretty-prints calendars
pub async fn print_calendar_list<C>(cals: &HashMap<Url, Arc<Mutex<C>>>)
where
    C: CompleteCalendar,
{
    for (url, cal) in cals {
        println!("CAL {} ({})", cal.lock().unwrap().name(), url);
        match cal.lock().unwrap().get_items().await {
            Err(_err) => continue,
            Ok(map) => {
                for (_, item) in map {
                    print_task(item);
                }
            }
        }
    }
}

/// A debug utility that pretty-prints calendars
pub async fn print_dav_calendar_list<C>(cals: &HashMap<Url, Arc<Mutex<C>>>)
where
    C: DavCalendar,
{
    for (url, cal) in cals {
        println!("CAL {} ({})", cal.lock().unwrap().name(), url);
        match cal.lock().unwrap().get_item_version_tags().await {
            Err(_err) => continue,
            Ok(map) => {
                for (url, version_tag) in map {
                    println!("    * {} (version {:?})", url, version_tag);
                }
            }
        }
    }
}

pub fn print_task(item: &Item) {
    match item {
        Item::Task(task) => {
            let completion = if task.completed() { "✓" } else { " " };
            let sync = match task.sync_status() {
                SyncStatus::NotSynced => ".",
                SyncStatus::Synced(_) => "=",
                SyncStatus::LocallyModified(_) => "~",
                SyncStatus::LocallyDeleted(_) => "x",
            };
            println!("    {}{} {}\t{}", completion, sync, task.name(), task.url());
        }
        _ => return,
    }
}

/// Compare keys of two hashmaps for equality
pub fn keys_are_the_same<T, U, V>(left: &HashMap<T, U>, right: &HashMap<T, V>) -> bool
where
    T: Hash + Eq + Clone + std::fmt::Display,
{
    if left.len() != right.len() {
        log::debug!("Count of keys mismatch: {} and {}", left.len(), right.len());
        return false;
    }

    let keys_l: HashSet<T> = left.keys().cloned().collect();
    let keys_r: HashSet<T> = right.keys().cloned().collect();
    let result = keys_l == keys_r;
    if result == false {
        log::debug!("Keys of a map mismatch");
        for key in keys_l {
            log::debug!("   left: {}", key);
        }
        log::debug!("RIGHT:");
        for key in keys_r {
            log::debug!("  right: {}", key);
        }
    }
    result
}

/// Wait for the user to press enter
pub fn pause() {
    let mut stdout = stdout();
    stdout.write_all(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read_exact(&mut [0]).unwrap();
}

/// Generate a random URL with a given prefix
pub fn random_url(parent_calendar: &Url) -> Url {
    let random = uuid::Uuid::new_v4().to_hyphenated().to_string();
    parent_calendar.join(&random).unwrap(/* this cannot panic since we've just created a string that is a valid URL */)
}
