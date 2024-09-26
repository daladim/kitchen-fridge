//! Some utility functions

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{self};
use std::hash::Hash;
use std::io::{stdin, stdout, Read, Write};
use std::sync::{Arc, Mutex};

use prop::{print_property, Property};
use serde::{Deserialize, Serialize};
use sync::Syncable;
use url::Url;

use crate::traits::CompleteCalendar;
use crate::traits::DavCalendar;
use crate::Item;

pub mod prop;
pub(crate) mod req;
pub mod sync;
pub(crate) mod xml;

/// A debug utility that pretty-prints calendars
pub async fn print_calendar_list<C>(cals: &HashMap<Url, Arc<Mutex<C>>>)
where
    C: CompleteCalendar,
{
    let ordered = {
        let mut v: Vec<(&Url, &Arc<Mutex<C>>)> = cals.iter().collect();
        v.sort_by_key(|x| x.0);
        v
    };

    for (url, cal) in ordered {
        println!("CAL {} ({})", cal.lock().unwrap().name(), url);
        match cal.lock().unwrap().get_items().await {
            Err(_err) => continue,
            Ok(map) => {
                for (_, item) in map {
                    print_task(item);
                }
            }
        }

        for prop in cal.lock().unwrap().get_properties().await.values() {
            print_property(prop);
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
    if let Item::Task(task) = item {
        let completion = if task.completed() { "✓" } else { " " };
        let sync = task.sync_status().symbol();
        println!("    {}{} {}\t{}", completion, sync, task.name(), task.url());
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
    if !result {
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

/// Generate a random NamespacedName, under a namespace we control
pub fn random_nsn() -> NamespacedName {
    NamespacedName {
        xmlns: "https://github.com/daladim/kitchen-fridge/__test_xmlns__/".to_string(),
        name: uuid::Uuid::new_v4().to_hyphenated().to_string(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct NamespacedName {
    pub xmlns: String,
    pub name: String,
}
impl NamespacedName {
    pub fn new<S1: ToString, S2: ToString>(xmlns: S1, name: S2) -> Self {
        Self {
            xmlns: xmlns.to_string(),
            name: name.to_string(),
        }
    }

    /// Uses namespace mappings to simplify the representation of this name
    /// For example, https://example.com/api/item becomes b:item if namespace https://example.com/api/ has symbol b in the namespace mapping
    pub fn with_symbolized_prefix(&self, namespaces: &Namespaces) -> String {
        let sym = namespaces.sym(&self.xmlns).unwrap();
        format!("{}:{}", sym, self.name)
    }
}
impl fmt::Display for NamespacedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.xmlns.as_str())?;
        fmt::Write::write_char(f, ':')?;
        f.write_str(self.name.as_str())
    }
}
impl From<Property> for NamespacedName {
    fn from(value: Property) -> Self {
        value.nsn().clone()
    }
}
impl PartialOrd for NamespacedName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for NamespacedName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.xmlns.cmp(&other.xmlns) {
            std::cmp::Ordering::Equal => self.name.cmp(&other.name),
            c => c,
        }
    }
}

/// Utility to track XML namespace symbol mappings, as used in xmlns attribute declarations
///
/// Includes a default mapping of xmlns:d="DAV:"
pub struct Namespaces {
    available_syms: VecDeque<char>,
    mapping: HashMap<String, char>,
}

impl Namespaces {
    pub fn new() -> Self {
        let mut mapping = HashMap::new();
        mapping.insert("DAV:".into(), 'd');

        Self {
            available_syms: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcefghijklmnopqrstuvwxyz" //NOTE the missing 'd'
                .chars()
                .collect(),
            mapping,
        }
    }

    /// Maps the namespace to an unassigned symbol and returns it
    pub fn add<S: ToString>(&mut self, ns: S) -> char {
        let sym = self
            .available_syms
            .pop_back()
            .expect("Ran out of namespace symbols");

        self.mapping.insert(ns.to_string(), sym);

        sym
    }

    pub fn decl(&self) -> String {
        let mut s = String::new();
        for (k, v) in &self.mapping {
            s.push(' ');
            s.push_str("xmlns:");
            s.push(*v);
            s.push('=');
            s.push('"');
            s.push_str(k.as_str());
            s.push('"');
        }
        s
    }

    pub fn sym(&self, ns: &String) -> Option<char> {
        self.mapping.get(ns).cloned()
    }

    pub fn dav_sym(&self) -> char {
        self.mapping[&"DAV:".to_string()]
    }
}
