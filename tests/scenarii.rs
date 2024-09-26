//! Multiple scenarios that are performed to test sync operations correctly work
//!
//! This module creates test data.
//! To do so, "scenarii" are defined. A scenario contains an inital state before sync, changes made either on the local or remote side, then the expected final state that should be present in both sources after sync.
//!
//! This module builds actual CalDAV sources (actually [`crate::cache::Cache`]s, that can also mock what would be [`crate::client::Client`]s in a real program) and [`crate::provider::Provider]`s that contain this data
//!
//! This module can also check the sources after a sync contain the actual data we expect
#![cfg(feature = "local_calendar_mocks_remote_calendars")]

use kitchen_fridge::error::KFResult;
use kitchen_fridge::utils::prop::Property;
use kitchen_fridge::utils::sync::{SyncStatus, Syncable, VersionTag};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use url::Url;

use chrono::Utc;

use kitchen_fridge::cache::Cache;
use kitchen_fridge::calendar::cached_calendar::CachedCalendar;
use kitchen_fridge::calendar::SupportedComponents;
use kitchen_fridge::mock_behaviour::MockBehaviour;
use kitchen_fridge::provider::Provider;
use kitchen_fridge::task::CompletionStatus;
use kitchen_fridge::traits::BaseCalendar;
use kitchen_fridge::traits::CalDavSource;
use kitchen_fridge::traits::CompleteCalendar;
use kitchen_fridge::traits::DavCalendar;
use kitchen_fridge::utils::{random_nsn, random_url, NamespacedName};
use kitchen_fridge::Item;
use kitchen_fridge::Task;

pub struct ItemState {
    // TODO: if/when this crate supports Events as well, we could add such events here
    /// The calendar it is in
    calendar: Url,
    /// Its name
    name: String,
    /// Its completion status
    completed: bool,
}

#[derive(Debug)]
pub enum LocatedState<S> {
    /// Item does not exist yet or does not exist anymore
    None,
    /// Item is only in the local source
    Local(S),
    /// Item is only in the remote source
    Remote(S),
    /// Item is synced at both locations,
    BothSynced(S),
}

pub enum ItemChange {
    Rename(String),
    SetCompletion(bool),
    Create(Url, Item),
    /// "remove" means "mark for deletion" in the local calendar, or "immediately delete" on the remote calendar
    Remove,
    // ChangeCalendar(Url) is useless, as long as changing a calendar is implemented as "delete in one calendar and re-create it in another one"
}

/// Like Property but doesn't track its own sync status, and says which calendar it applies to
#[derive(Debug)]
pub struct PropState {
    /// The calendar the property is set on
    calendar: Url,
    nsn: NamespacedName,
    value: String,
}

#[derive(Debug)]
pub enum PropChange {
    /// Set the property value
    ///
    /// It's an error to change the nsn
    Set(PropState),

    /// Remove the property
    Remove,
}

pub struct ItemScenario {
    /// The URL of the item
    url: Url,
    initial_state: LocatedState<ItemState>,
    local_changes_to_apply: Vec<ItemChange>,
    remote_changes_to_apply: Vec<ItemChange>,
    after_sync: LocatedState<ItemState>,
}

#[derive(Debug)]
pub struct PropScenario {
    /// The namespace and element name of the property
    nsn: NamespacedName,
    initial_state: LocatedState<PropState>,
    local_changes_to_apply: Vec<PropChange>,
    remote_changes_to_apply: Vec<PropChange>,
    after_sync: LocatedState<PropState>,
}

/// Generate the scenarii required for the following test:
/// * At the last sync: both sources had A, B, C, D, E, F, G, H, I, J, K, L, M✓, N✓, O✓, P✓ at last sync
///   A-F are in a calendar, G-M are in a second one, and in a third calendar from N on
///
/// * Before the newer sync, this will be the content of the sources:
///     * cache:  A, B,    D', E,  F'', G , H✓, I✓, J✓,       M,  N✓, O, P',     R
///     * server: A,    C, D,  E', F',  G✓, H , I',     K✓,   M✓, N , O, P✓,  Q
///
/// Hence, here is the expected result after the sync:
///     * both:   A,       D', E', F',  G✓, H✓, I',     K✓,   M,  N , O, P',  Q, R
///
/// Notes:
/// * X': name has been modified since the last sync
/// * X'/X'': name conflict
/// * X✓: task has been marked as completed
pub fn item_scenarii_basic() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let first_cal = "https://some.calend.ar/calendar-1/".parse().unwrap();
    let second_cal = "https://some.calend.ar/calendar-2/".parse().unwrap();
    let third_cal = "https://some.calend.ar/calendar-3/".parse().unwrap();

    tasks.push(ItemScenario {
        url: random_url(&first_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task A"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task A"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&first_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task B"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: vec![ItemChange::Remove],
        after_sync: LocatedState::None,
    });

    tasks.push(ItemScenario {
        url: random_url(&first_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task C"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::Remove],
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::None,
    });

    tasks.push(ItemScenario {
        url: random_url(&first_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task D"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::Rename(String::from("Task D, locally renamed"))],
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task D, locally renamed"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&first_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task E"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: vec![ItemChange::Rename(String::from("Task E, remotely renamed"))],
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task E, remotely renamed"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&first_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task F"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::Rename(String::from("Task F, locally renamed"))],
        remote_changes_to_apply: vec![ItemChange::Rename(String::from("Task F, remotely renamed"))],
        // Conflict: the server wins
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: first_cal.clone(),
            name: String::from("Task F, remotely renamed"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&second_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task G"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: vec![ItemChange::SetCompletion(true)],
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task G"),
            completed: true,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&second_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task H"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::SetCompletion(true)],
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task H"),
            completed: true,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&second_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task I"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::SetCompletion(true)],
        remote_changes_to_apply: vec![ItemChange::Rename(String::from("Task I, remotely renamed"))],
        // Conflict, the server wins
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task I, remotely renamed"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&second_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task J"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::SetCompletion(true)],
        remote_changes_to_apply: vec![ItemChange::Remove],
        after_sync: LocatedState::None,
    });

    tasks.push(ItemScenario {
        url: random_url(&second_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task K"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::Remove],
        remote_changes_to_apply: vec![ItemChange::SetCompletion(true)],
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task K"),
            completed: true,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&second_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task L"),
            completed: false,
        }),
        local_changes_to_apply: vec![ItemChange::Remove],
        remote_changes_to_apply: vec![ItemChange::Remove],
        after_sync: LocatedState::None,
    });

    tasks.push(ItemScenario {
        url: random_url(&second_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task M"),
            completed: true,
        }),
        local_changes_to_apply: vec![ItemChange::SetCompletion(false)],
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: second_cal.clone(),
            name: String::from("Task M"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&third_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task N"),
            completed: true,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: vec![ItemChange::SetCompletion(false)],
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task N"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&third_cal),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task O"),
            completed: true,
        }),
        local_changes_to_apply: vec![ItemChange::SetCompletion(false)],
        remote_changes_to_apply: vec![ItemChange::SetCompletion(false)],
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task O"),
            completed: false,
        }),
    });

    let url_p = random_url(&third_cal);
    tasks.push(ItemScenario {
        url: url_p.clone(),
        initial_state: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task P"),
            completed: true,
        }),
        local_changes_to_apply: vec![
            ItemChange::Rename(String::from("Task P, locally renamed and un-completed")),
            ItemChange::SetCompletion(false),
        ],
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task P, locally renamed and un-completed"),
            completed: false,
        }),
    });

    let url_q = random_url(&third_cal);
    tasks.push(ItemScenario {
        url: url_q.clone(),
        initial_state: LocatedState::None,
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: vec![ItemChange::Create(
            third_cal.clone(),
            Item::Task(Task::new_with_parameters(
                String::from("Task Q, created on the server"),
                url_q.to_string(),
                url_q,
                CompletionStatus::Uncompleted,
                SyncStatus::random_synced(),
                Some(Utc::now()),
                Utc::now(),
                "prod_id".to_string(),
                Vec::new(),
                Vec::new(),
            )),
        )],
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task Q, created on the server"),
            completed: false,
        }),
    });

    let url_r = random_url(&third_cal);
    tasks.push(ItemScenario {
        url: url_r.clone(),
        initial_state: LocatedState::None,
        local_changes_to_apply: vec![ItemChange::Create(
            third_cal.clone(),
            Item::Task(Task::new_with_parameters(
                String::from("Task R, created locally"),
                url_r.to_string(),
                url_r,
                CompletionStatus::Uncompleted,
                SyncStatus::NotSynced,
                Some(Utc::now()),
                Utc::now(),
                "prod_id".to_string(),
                Vec::new(),
                Vec::new(),
            )),
        )],
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: third_cal.clone(),
            name: String::from("Task R, created locally"),
            completed: false,
        }),
    });

    tasks
}

/// This scenario basically checks a first sync to an empty local cache
pub fn item_scenarii_first_sync_to_local() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let cal1 = "https://some.calend.ar/first/".parse().unwrap();
    let cal2 = "https://some.calend.ar/second/".parse().unwrap();

    tasks.push(ItemScenario {
        url: random_url(&cal1),
        initial_state: LocatedState::Remote(ItemState {
            calendar: cal1.clone(),
            name: String::from("Task A1"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: cal1.clone(),
            name: String::from("Task A1"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&cal2),
        initial_state: LocatedState::Remote(ItemState {
            calendar: cal2.clone(),
            name: String::from("Task A2"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: cal2.clone(),
            name: String::from("Task A2"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&cal1),
        initial_state: LocatedState::Remote(ItemState {
            calendar: cal1.clone(),
            name: String::from("Task B1"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: cal1.clone(),
            name: String::from("Task B1"),
            completed: false,
        }),
    });

    tasks
}

pub fn prop_scenarii_first_sync_to_local() -> Vec<PropScenario> {
    let mut tasks = Vec::new();

    let cal1: Url = "https://some.calend.ar/first".parse().unwrap();
    let cal2: Url = "https://some.calend.ar/second".parse().unwrap();

    {
        let nsn = random_nsn();
        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::Remote(PropState {
                calendar: cal1.clone(),
                nsn: nsn.clone(),
                value: String::from("Value A1"),
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced(PropState {
                calendar: cal1.clone(),
                nsn,
                value: String::from("Value A1"),
            }),
        });
    }

    {
        let nsn = random_nsn();
        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::Remote(PropState {
                calendar: cal2.clone(),
                nsn: nsn.clone(),
                value: String::from("Value A2"),
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced(PropState {
                calendar: cal2.clone(),
                nsn,
                value: String::from("Value A2"),
            }),
        });
    }

    {
        let nsn = random_nsn();
        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::Remote(PropState {
                calendar: cal1.clone(),
                nsn: nsn.clone(),
                value: String::from("Value B1"),
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced(PropState {
                calendar: cal1.clone(),
                nsn,
                value: String::from("Value B1"),
            }),
        });
    }

    tasks
}

/// This scenario basically checks a first sync to an empty server
pub fn item_scenarii_first_sync_to_server() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let cal3 = "https://some.calend.ar/third/".parse().unwrap();
    let cal4 = "https://some.calend.ar/fourth/".parse().unwrap();

    tasks.push(ItemScenario {
        url: random_url(&cal3),
        initial_state: LocatedState::Local(ItemState {
            calendar: cal3.clone(),
            name: String::from("Task A3"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: cal3.clone(),
            name: String::from("Task A3"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&cal4),
        initial_state: LocatedState::Local(ItemState {
            calendar: cal4.clone(),
            name: String::from("Task A4"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: cal4.clone(),
            name: String::from("Task A4"),
            completed: false,
        }),
    });

    tasks.push(ItemScenario {
        url: random_url(&cal3),
        initial_state: LocatedState::Local(ItemState {
            calendar: cal3.clone(),
            name: String::from("Task B3"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: cal3.clone(),
            name: String::from("Task B3"),
            completed: false,
        }),
    });

    tasks
}

/// This scenario basically checks a first sync to an empty server
pub fn prop_scenarii_first_sync_to_server() -> Vec<PropScenario> {
    let mut tasks = Vec::new();

    let cal3: Url = "https://some.calend.ar/third/".parse().unwrap();
    let cal4: Url = "https://some.calend.ar/fourth/".parse().unwrap();

    {
        let nsn = random_nsn();
        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::Local(PropState {
                calendar: cal3.clone(),
                nsn: nsn.clone(),
                value: String::from("Value A3"),
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced(PropState {
                calendar: cal3.clone(),
                nsn: nsn.clone(),
                value: String::from("Value A3"),
            }),
        });
    }

    {
        let nsn = random_nsn();
        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::Local(PropState {
                calendar: cal4.clone(),
                nsn: nsn.clone(),
                value: String::from("Value A4"),
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced(PropState {
                calendar: cal4.clone(),
                nsn: nsn.clone(),
                value: String::from("Value A4"),
            }),
        });
    }

    {
        let nsn = random_nsn();
        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::Local(PropState {
                calendar: cal3.clone(),
                nsn: nsn.clone(),
                value: String::from("Value B3"),
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced(PropState {
                calendar: cal3.clone(),
                nsn: nsn.clone(),
                value: String::from("Value B3"),
            }),
        });
    }

    tasks
}

/// This scenario tests a task added and deleted before a sync happens
pub fn item_scenarii_transient_task() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let cal = "https://some.calend.ar/transient/".parse().unwrap();

    tasks.push(ItemScenario {
        url: random_url(&cal),
        initial_state: LocatedState::Local(ItemState {
            calendar: cal.clone(),
            name: String::from("A task, so that the calendar actually exists"),
            completed: false,
        }),
        local_changes_to_apply: Vec::new(),
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::BothSynced(ItemState {
            calendar: cal.clone(),
            name: String::from("A task, so that the calendar actually exists"),
            completed: false,
        }),
    });

    let url_transient = random_url(&cal);
    tasks.push(ItemScenario {
        url: url_transient.clone(),
        initial_state: LocatedState::None,
        local_changes_to_apply: vec![
            ItemChange::Create(
                cal,
                Item::Task(Task::new_with_parameters(
                    String::from("A transient task that will be deleted before the sync"),
                    url_transient.to_string(),
                    url_transient,
                    CompletionStatus::Uncompleted,
                    SyncStatus::NotSynced,
                    Some(Utc::now()),
                    Utc::now(),
                    "prod_id".to_string(),
                    Vec::new(),
                    Vec::new(),
                )),
            ),
            ItemChange::Rename(String::from("A new name")),
            ItemChange::SetCompletion(true),
            ItemChange::Remove,
        ],
        remote_changes_to_apply: Vec::new(),
        after_sync: LocatedState::None,
    });

    tasks
}

/// This scenario tests a task added and deleted before a sync happens
pub fn prop_scenarii_transient_prop() -> Vec<PropScenario> {
    let mut tasks = Vec::new();

    let cal: Url = "https://some.calend.ar/transient_prop/".parse().unwrap();

    {
        let nsn = random_nsn();
        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::Local(PropState {
                calendar: cal.clone(),
                nsn: nsn.clone(),
                value: String::from("A prop, so that the calendar actually exists"),
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced(PropState {
                calendar: cal.clone(),
                nsn: nsn.clone(),
                value: String::from("A prop, so that the calendar actually exists"),
            }),
        });
    }

    {
        let nsn = random_nsn();

        tasks.push(PropScenario {
            nsn: nsn.clone(),
            initial_state: LocatedState::None,
            local_changes_to_apply: vec![
                PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("A transient task that will be deleted before the sync"),
                }),
                PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("A new name"),
                }),
                PropChange::Remove,
            ],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::None,
        });
    }

    tasks
}

/// Generate the scenarii required for the following test:
/// At last sync, we had three calendars with the following properties:
///  1: A, B, C, D, E, F
///  2: L
///
/// Before the newer sync, this will be the content of the sources:
/// * cache:  1. A, B,    D', E,  F''    2.      3.    R
/// * server: 1. A,    C, D,  E', F'     2.      3. Q
///
/// Hence, here is the expected result after the sync:
///  1. A,       D', E', F'
///  2.
///  3. Q, R
///
/// Notes:
/// * X': value has been modified since the last sync
/// * X'/X'': value conflict
pub fn prop_scenarii_basic() -> Vec<PropScenario> {
    let mut tasks = Vec::new();

    let n = |name: String| NamespacedName {
        xmlns: "https://github.com/daladim/kitchen-fridge/__test_xmlns__/".to_string(),
        name,
    };

    {
        let cal: Url = "https://some.calend.ar/calendar-1/".parse().unwrap();
        {
            let nsn = n("a".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value A"),
                }),
                local_changes_to_apply: Vec::new(),
                remote_changes_to_apply: Vec::new(),
                after_sync: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value A"),
                }),
            });
        }

        {
            let nsn = n("b".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value B"),
                }),
                local_changes_to_apply: Vec::new(),
                remote_changes_to_apply: vec![PropChange::Remove],
                after_sync: LocatedState::None,
            });
        }

        {
            let nsn = n("c".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value C"),
                }),
                local_changes_to_apply: vec![PropChange::Remove],
                remote_changes_to_apply: Vec::new(),
                after_sync: LocatedState::None,
            });
        }
        {
            let nsn = n("d".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value D"),
                }),
                local_changes_to_apply: vec![PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value D, locally changed"),
                })],

                remote_changes_to_apply: Vec::new(),
                after_sync: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value D, locally changed"),
                }),
            });
        }

        {
            let nsn = n("e".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value E"),
                }),
                local_changes_to_apply: Vec::new(),
                remote_changes_to_apply: vec![PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value E, remotely changed"),
                })],
                after_sync: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value E, remotely changed"),
                }),
            });
        }
        {
            let nsn = n("f".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value F"),
                }),
                local_changes_to_apply: vec![PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value F, locally changed"),
                })],
                remote_changes_to_apply: vec![PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value F, remotely changed"),
                })],
                // Conflict: the server wins
                after_sync: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value F, remotely changed"),
                }),
            });
        }
    }

    {
        let cal: Url = "https://some.calend.ar/calendar-2/".parse().unwrap();
        {
            let nsn = n("l".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value L"),
                }),
                local_changes_to_apply: vec![PropChange::Remove],
                remote_changes_to_apply: vec![PropChange::Remove],
                after_sync: LocatedState::None,
            });
        }
    }

    {
        let cal: Url = "https://some.calend.ar/calendar-3/".parse().unwrap();
        {
            let nsn = n("q".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::None,
                local_changes_to_apply: Vec::new(),
                remote_changes_to_apply: vec![PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: "Value Q, created on the server".to_string(),
                })],
                after_sync: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value Q, created on the server"),
                }),
            });
        }

        {
            let nsn = n("r".into());
            tasks.push(PropScenario {
                nsn: nsn.clone(),
                initial_state: LocatedState::None,
                local_changes_to_apply: vec![PropChange::Set(PropState {
                    calendar: cal.clone(),
                    nsn: nsn.clone(),
                    value: String::from("Value R, created locally"),
                })],
                remote_changes_to_apply: Vec::new(),
                after_sync: LocatedState::BothSynced(PropState {
                    calendar: cal.clone(),
                    nsn,
                    value: String::from("Value R, created locally"),
                }),
            });
        }
    }

    tasks
}

/// Build a `Provider` that contains the data (defined in the given scenarii) before sync
pub async fn populate_test_provider_before_sync(
    item_scenarii: &[ItemScenario],
    prop_scenarii: &[PropScenario],
    mock_behaviour: Arc<Mutex<MockBehaviour>>,
) -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    let mut provider =
        populate_test_provider(item_scenarii, prop_scenarii, mock_behaviour, false).await;
    apply_changes_on_provider(&mut provider, item_scenarii, prop_scenarii).await;
    provider
}

/// Build a `Provider` that contains the data (defined in the given scenarii) after sync
pub async fn populate_test_provider_after_sync(
    item_scenarii: &[ItemScenario],
    prop_scenarii: &[PropScenario],
    mock_behaviour: Arc<Mutex<MockBehaviour>>,
) -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    populate_test_provider(item_scenarii, prop_scenarii, mock_behaviour, true).await
}

async fn populate_test_provider(
    item_scenarii: &[ItemScenario],
    prop_scenarii: &[PropScenario],
    mock_behaviour: Arc<Mutex<MockBehaviour>>,
    populate_for_final_state: bool,
) -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    let mut local = Cache::new(&PathBuf::from(String::from("test_cache/local/")));
    let mut remote = Cache::new(&PathBuf::from(String::from("test_cache/remote/")));
    remote.set_mock_behaviour(Some(mock_behaviour));

    // Create the initial state, as if we synced both sources in a given state
    for item in item_scenarii {
        let required_state = if populate_for_final_state {
            &item.after_sync
        } else {
            &item.initial_state
        };
        let (state, sync_status) = match required_state {
            LocatedState::None => continue,
            LocatedState::Local(s) => {
                assert!(
                    !populate_for_final_state,
                    "You are not supposed to expect an item in this state after sync"
                );
                (s, SyncStatus::NotSynced)
            }
            LocatedState::Remote(s) => {
                assert!(
                    !populate_for_final_state,
                    "You are not supposed to expect an item in this state after sync"
                );
                (s, SyncStatus::random_synced())
            }
            LocatedState::BothSynced(s) => (s, SyncStatus::random_synced()),
        };

        let now = Utc::now();
        let completion_status = match state.completed {
            false => CompletionStatus::Uncompleted,
            true => CompletionStatus::Completed(Some(now)),
        };

        let new_item = Item::Task(Task::new_with_parameters(
            state.name.clone(),
            item.url.to_string(),
            item.url.clone(),
            completion_status,
            sync_status,
            Some(now),
            now,
            "prod_id".to_string(),
            Vec::new(),
            Vec::new(),
        ));

        match required_state {
            LocatedState::None => panic!("Should not happen, we've continued already"),
            LocatedState::Local(s) => {
                get_or_insert_calendar(&mut local, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .add_item(new_item)
                    .await
                    .unwrap();
            }
            LocatedState::Remote(s) => {
                get_or_insert_calendar(&mut remote, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .add_item(new_item)
                    .await
                    .unwrap();
            }
            LocatedState::BothSynced(s) => {
                get_or_insert_calendar(&mut local, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .add_item(new_item.clone())
                    .await
                    .unwrap();
                get_or_insert_calendar(&mut remote, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .add_item(new_item)
                    .await
                    .unwrap();
            }
        }
    }

    for prop in prop_scenarii {
        let required_state = if populate_for_final_state {
            &prop.after_sync
        } else {
            &prop.initial_state
        };
        let (state, sync_status) = match required_state {
            LocatedState::None => continue,
            LocatedState::Local(s) => {
                assert!(
                    !populate_for_final_state,
                    "You are not supposed to expect prop in this state after sync"
                );
                (s, SyncStatus::NotSynced)
            }
            LocatedState::Remote(s) => {
                assert!(
                    !populate_for_final_state,
                    "You are not supposed to expect a prop in this state after sync"
                );
                (s, SyncStatus::Synced(VersionTag::from(s.value.clone())))
            }
            LocatedState::BothSynced(s) => {
                (s, SyncStatus::Synced(VersionTag::from(s.value.clone())))
            }
        };

        let new_prop = {
            let mut p = Property::new(
                state.nsn.xmlns.clone(),
                state.nsn.name.clone(),
                state.value.clone(),
            );
            p.set_sync_status(sync_status);
            p
        };

        match required_state {
            LocatedState::None => panic!("Should not happen, we've continued already"),
            LocatedState::Local(s) => {
                log::debug!("Setting local to {:?}", new_prop);
                get_or_insert_calendar(&mut local, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_property(new_prop.clone())
                    .await
                    .unwrap();
                debug_assert_eq!(
                    get_or_insert_calendar(&mut local, &s.calendar)
                        .await
                        .unwrap()
                        .lock()
                        .unwrap()
                        .get_property_by_name(new_prop.nsn())
                        .await,
                    Some(&new_prop)
                );
            }
            LocatedState::Remote(s) => {
                log::debug!("Setting remote to {:?}", new_prop);
                get_or_insert_calendar(&mut remote, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_property(new_prop.clone())
                    .await
                    .unwrap();
                debug_assert_eq!(
                    get_or_insert_calendar(&mut remote, &s.calendar)
                        .await
                        .unwrap()
                        .lock()
                        .unwrap()
                        .get_property_by_name(new_prop.nsn())
                        .await,
                    Some(&new_prop)
                );
            }
            LocatedState::BothSynced(s) => {
                log::debug!("Setting local and remote to {:?}", new_prop);
                get_or_insert_calendar(&mut local, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_property(new_prop.clone())
                    .await
                    .unwrap();
                get_or_insert_calendar(&mut remote, &s.calendar)
                    .await
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_property(new_prop.clone())
                    .await
                    .unwrap();

                debug_assert_eq!(
                    get_or_insert_calendar(&mut local, &s.calendar)
                        .await
                        .unwrap()
                        .lock()
                        .unwrap()
                        .get_property_by_name(new_prop.nsn())
                        .await,
                    Some(&new_prop)
                );
                debug_assert_eq!(
                    get_or_insert_calendar(&mut remote, &s.calendar)
                        .await
                        .unwrap()
                        .lock()
                        .unwrap()
                        .get_property_by_name(new_prop.nsn())
                        .await,
                    Some(&new_prop)
                );
            }
        }
    }
    Provider::new(remote, local)
}

/// Apply `local_changes_to_apply` and `remote_changes_to_apply` to a provider that contains data before sync
async fn apply_changes_on_provider(
    provider: &mut Provider<Cache, CachedCalendar, Cache, CachedCalendar>,
    item_scenarii: &[ItemScenario],
    prop_scenarii: &[PropScenario],
) {
    // Apply changes to each item
    for item in item_scenarii {
        let initial_calendar_url = match &item.initial_state {
            LocatedState::None => None,
            LocatedState::Local(state) => Some(state.calendar.clone()),
            LocatedState::Remote(state) => Some(state.calendar.clone()),
            LocatedState::BothSynced(state) => Some(state.calendar.clone()),
        };

        let mut calendar_url = initial_calendar_url.clone();
        for local_change in &item.local_changes_to_apply {
            calendar_url = Some(
                apply_item_change(
                    provider.local(),
                    calendar_url,
                    &item.url,
                    local_change,
                    false,
                )
                .await,
            );
        }

        let mut calendar_url = initial_calendar_url;
        for remote_change in &item.remote_changes_to_apply {
            calendar_url = Some(
                apply_item_change(
                    provider.remote(),
                    calendar_url,
                    &item.url,
                    remote_change,
                    true,
                )
                .await,
            );
        }
    }
    // Apply changes to each prop
    for prop in prop_scenarii {
        log::debug!("Applying prop scenario: {:?}\n", prop);
        let initial_calendar_url = match &prop.initial_state {
            LocatedState::None => None,
            LocatedState::Local(state) => Some(state.calendar.clone()),
            LocatedState::Remote(state) => Some(state.calendar.clone()),
            LocatedState::BothSynced(state) => Some(state.calendar.clone()),
        };

        {
            let mut calendar_url = initial_calendar_url.clone();
            for local_change in &prop.local_changes_to_apply {
                if let PropChange::Set(s) = local_change {
                    assert_eq!(prop.nsn, s.nsn);
                }

                if let Some(calendar_url) = calendar_url.as_ref() {
                    let cal = provider.local().get_calendar(calendar_url).await.unwrap();
                    let cal = cal.lock().unwrap();

                    assert!(cal.get_property_by_name(&prop.nsn).await.is_some());
                }

                calendar_url = Some(
                    apply_prop_change(
                        provider.local(),
                        calendar_url,
                        &prop.nsn,
                        local_change,
                        false,
                    )
                    .await,
                );
            }
        }

        let mut calendar_url = initial_calendar_url;
        for remote_change in &prop.remote_changes_to_apply {
            calendar_url = Some(
                apply_prop_change(
                    provider.remote(),
                    calendar_url,
                    &prop.nsn,
                    remote_change,
                    true,
                )
                .await,
            );
        }
    }
}

async fn get_or_insert_calendar(
    source: &mut Cache,
    url: &Url,
) -> KFResult<Arc<Mutex<CachedCalendar>>> {
    match source.get_calendar(url).await {
        Some(cal) => Ok(cal),
        None => {
            let new_name = format!("Test calendar for URL {}", url);
            let supported_components = SupportedComponents::TODO;
            let color = csscolorparser::parse("#ff8000").unwrap(); // TODO: we should rather have specific colors, depending on the calendars

            source
                .create_calendar(
                    url.clone(),
                    new_name.to_string(),
                    supported_components,
                    Some(color),
                )
                .await
        }
    }
}

/// Apply a single change on a given source, and returns the calendar URL that was modified
async fn apply_item_change<S, C>(
    source: &S,
    calendar_url: Option<Url>,
    item_url: &Url,
    change: &ItemChange,
    is_remote: bool,
) -> Url
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match calendar_url {
        Some(cal) => {
            apply_changes_on_an_existing_item(source, &cal, item_url, change, is_remote).await;
            cal
        }
        None => create_test_item(source, change).await,
    }
}

/// Apply a single change on a given source, and returns the calendar URL that was modified
async fn apply_prop_change<S, C>(
    source: &S,
    calendar_url: Option<Url>,
    nsn: &NamespacedName,
    change: &PropChange,
    is_remote: bool,
) -> Url
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match calendar_url {
        Some(cal) => {
            apply_changes_on_an_existing_prop(source, &cal, nsn, change, is_remote).await;
            cal
        }
        None => create_test_prop(source, change).await,
    }
}

async fn apply_changes_on_an_existing_item<S, C>(
    source: &S,
    calendar_url: &Url,
    item_url: &Url,
    change: &ItemChange,
    is_remote: bool,
) where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    let cal = source.get_calendar(calendar_url).await.unwrap();
    let mut cal = cal.lock().unwrap();
    let task = cal
        .get_item_by_url_mut(item_url)
        .await
        .unwrap()
        .unwrap_task_mut();

    match change {
        ItemChange::Rename(new_name) => {
            if is_remote {
                task.mock_remote_calendar_set_name(new_name.clone());
            } else {
                task.set_name(new_name.clone());
            }
        }
        ItemChange::SetCompletion(new_status) => {
            let completion_status = match new_status {
                false => CompletionStatus::Uncompleted,
                true => CompletionStatus::Completed(Some(Utc::now())),
            };
            if is_remote {
                task.mock_remote_calendar_set_completion_status(completion_status);
            } else {
                task.set_completion_status(completion_status);
            }
        }
        ItemChange::Remove => {
            match is_remote {
                false => cal.mark_item_for_deletion(item_url).await.unwrap(),
                true => cal.delete_item(item_url).await.unwrap(),
            };
        }
        ItemChange::Create(_calendar_url, _item) => {
            panic!("This function only handles already existing items");
        }
    }
}

async fn apply_changes_on_an_existing_prop<S, C>(
    source: &S,
    calendar_url: &Url,
    nsn: &NamespacedName,
    change: &PropChange,
    is_remote: bool,
) where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    let cal = source.get_calendar(calendar_url).await.unwrap();
    let mut cal = cal.lock().unwrap();
    let prop = cal.get_property_by_name_mut(nsn).await.unwrap_or_else(|| {
        panic!(
            "Couldn't get supposedly-existing property {} while applying change {:?}",
            nsn, change
        )
    });

    match change {
        PropChange::Set(s) => {
            debug_assert_eq!(prop.nsn(), &s.nsn);

            if is_remote {
                prop.mock_remote_calendar_set_value(s.value.clone());
            } else {
                prop.set_value(s.value.clone());
            }
        }
        PropChange::Remove => {
            match is_remote {
                false => cal.mark_prop_for_deletion(nsn).await.unwrap(),
                true => cal.delete_property(nsn).await.unwrap(),
            };
        }
    }
}

/// Create an item, and returns the URL of the calendar it was inserted in
async fn create_test_item<S, C>(source: &S, change: &ItemChange) -> Url
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match change {
        ItemChange::Rename(_) | ItemChange::SetCompletion(_) | ItemChange::Remove => {
            panic!("This function only creates items that do not exist yet");
        }
        ItemChange::Create(calendar_url, item) => {
            let cal = source.get_calendar(calendar_url).await.unwrap();
            cal.lock().unwrap().add_item(item.clone()).await.unwrap();
            calendar_url.clone()
        }
    }
}

/// Create a property, and returns the URL of the calendar it was added to
async fn create_test_prop<S, C>(source: &S, change: &PropChange) -> Url
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match change {
        PropChange::Remove => {
            panic!("This function only creates props that do not exist yet");
        }
        PropChange::Set(s) => {
            let cal = source.get_calendar(&s.calendar).await.unwrap();

            let prop = Property::new(s.nsn.xmlns.clone(), s.nsn.name.clone(), s.value.clone());

            log::debug!("Creating test prop {:?}\n", prop);
            cal.lock().unwrap().set_property(prop).await.unwrap();
            s.calendar.clone()
        }
    }
}
