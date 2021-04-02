//! Multiple scenarios that are performed to test sync operations correctly work
#![cfg(feature = "integration_tests")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::error::Error;

use my_tasks::calendar::CalendarId;
use my_tasks::calendar::SupportedComponents;
use my_tasks::traits::CalDavSource;
use my_tasks::traits::BaseCalendar;
use my_tasks::traits::CompleteCalendar;
use my_tasks::traits::DavCalendar;
use my_tasks::cache::Cache;
use my_tasks::Item;
use my_tasks::ItemId;
use my_tasks::SyncStatus;
use my_tasks::Task;
use my_tasks::calendar::cached_calendar::CachedCalendar;
use my_tasks::Provider;

pub enum LocatedState {
    /// Item does not exist yet or does not exist anymore
    None,
    /// Item is only in the local source
    Local(ItemState),
    /// Item is only in the remote source
    Remote(ItemState),
    /// Item is synced at both locations,
    BothSynced(ItemState),
}

pub struct ItemState {
    // TODO: if/when this crate supports Events as well, we could add such events here
    /// The calendar it is in
    calendar: CalendarId,
    /// Its name
    name: String,
    /// Its completion status
    completed: bool,
}

pub enum ChangeToApply {
    Rename(String),
    SetCompletion(bool),
    ChangeCalendar(CalendarId),
    Create(CalendarId, Item),
    /// "remove" means "mark for deletion" in the local calendar, or "immediately delete" on the remote calendar
    Remove,
}


pub struct ItemScenario {
    id: ItemId,
    before_sync: LocatedState,
    local_changes_to_apply:  Vec<ChangeToApply>,
    remote_changes_to_apply: Vec<ChangeToApply>,
    after_sync: LocatedState,
}

/// Populate sources with the following:
/// * At the last sync: both sources had A, B, C, D, E, F, G, H, I, J, K, L, M✓, N✓, O✓ at last sync
/// * Before the newer sync, this will be the content of the sources:
///     * cache:  A, B,    D', E,  F'', G , H✓, I✓, J✓,        M,  N✓, O, P,
///     * server: A,    C, D,  E', F',  G✓, H , I',      K✓,    M✓, N , O,   Q
///
/// Hence, here is the expected result after the sync:
///     * both:   A,       D', E', F',  G✓, H✓, I',      K✓,   M, N, O, P, Q
///
/// Notes:
/// * X': name has been modified since the last sync
/// * F'/F'': name conflict
/// * G✓: task has been marked as completed
pub fn basic_scenarii() -> Vec<ItemScenario> {
    let mut tasks = Vec::new();

    let main_cal = CalendarId::from("https://some.calend.ar/main/".parse().unwrap());

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task A"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task A"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task B"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::Remove],
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task C"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Remove],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task D"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task D, locally renamed"))],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task D, locally renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task E"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task E, remotely renamed"))],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task E, remotely renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task F"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task F, locally renamed"))],
            remote_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task F, remotely renamed"))],
            // Conflict: the server wins
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task F, remotely renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task G"),
                completed: false,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task G"),
                completed: true,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task H"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task H"),
                completed: true,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task I"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            remote_changes_to_apply: vec![ChangeToApply::Rename(String::from("Task I, remotely renamed"))],
            // Conflict, the server wins
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task I, remotely renamed"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task J"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            remote_changes_to_apply: vec![ChangeToApply::Remove],
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task K"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Remove],
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(true)],
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task L"),
                completed: false,
            }),
            local_changes_to_apply: vec![ChangeToApply::Remove],
            remote_changes_to_apply: vec![ChangeToApply::Remove],
            after_sync: LocatedState::None,
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task M"),
                completed: true,
            }),
            local_changes_to_apply: vec![ChangeToApply::SetCompletion(false)],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task M"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task N"),
                completed: true,
            }),
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(false)],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task N"),
                completed: false,
            }),
        }
    );

    tasks.push(
        ItemScenario {
            id: ItemId::random(),
            before_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task O"),
                completed: true,
            }),
            local_changes_to_apply:  vec![ChangeToApply::SetCompletion(false)],
            remote_changes_to_apply: vec![ChangeToApply::SetCompletion(false)],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task O"),
                completed: false,
            }),
        }
    );

    let id_p = ItemId::random();
    tasks.push(
        ItemScenario {
            id: id_p.clone(),
            before_sync: LocatedState::None,
            local_changes_to_apply: vec![ChangeToApply::Create(main_cal.clone(), Item::Task(
                Task::new(String::from("Task P, created locally"), id_p, SyncStatus::NotSynced, false )
            ))],
            remote_changes_to_apply: Vec::new(),
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task P, created locally"),
                completed: false,
            }),
        }
    );

    let id_q = ItemId::random();
    tasks.push(
        ItemScenario {
            id: id_q.clone(),
            before_sync: LocatedState::None,
            local_changes_to_apply: Vec::new(),
            remote_changes_to_apply: vec![ChangeToApply::Create(main_cal.clone(), Item::Task(
                Task::new(String::from("Task Q, created on the server"), id_q, SyncStatus::random_synced(), false )
            ))],
            after_sync: LocatedState::BothSynced( ItemState{
                calendar: main_cal.clone(),
                name: String::from("Task Q, created on the server"),
                completed: false,
            }),
        }
    );

    tasks
}

pub async fn populate_test_provider(scenarii: &[ItemScenario]) -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    let mut remote = Cache::new(&PathBuf::from(String::from("test_cache_remote/")));
    let mut local = Cache::new(&PathBuf::from(String::from("test_cache_local/")));

    // Create the initial state, as if we synced both sources in a given state
    for item in scenarii {
        let (state, sync_status) = match &item.before_sync {
            LocatedState::None => continue,
            LocatedState::Local(s) => (s, SyncStatus::NotSynced),
            LocatedState::Remote(s) => (s, SyncStatus::random_synced()),
            LocatedState::BothSynced(s) => (s, SyncStatus::random_synced()),
        };

        let new_item = Item::Task(
            Task::new(
                state.name.clone(),
                item.id.clone(),
                sync_status,
                state.completed,
            ));

        match &item.before_sync {
            LocatedState::None => panic!("Should not happen, we've continued already"),
            LocatedState::Local(s) => {
                get_or_insert_calendar(&mut local, &s.calendar).await.unwrap().lock().unwrap().add_item(new_item).await.unwrap();
            },
            LocatedState::Remote(s) => {
                get_or_insert_calendar(&mut remote, &s.calendar).await.unwrap().lock().unwrap().add_item(new_item).await.unwrap();
            },
            LocatedState::BothSynced(s) => {
                get_or_insert_calendar(&mut local, &s.calendar).await.unwrap().lock().unwrap().add_item(new_item.clone()).await.unwrap();
                get_or_insert_calendar(&mut remote, &s.calendar).await.unwrap().lock().unwrap().add_item(new_item).await.unwrap();
            },
        }
    }
    let provider = Provider::new(remote, local);


    // Apply changes to each item
    for item in scenarii {
        let initial_calendar_id = match &item.before_sync {
            LocatedState::None => None,
            LocatedState::Local(state) => Some(&state.calendar),
            LocatedState::Remote(state) => Some(&state.calendar),
            LocatedState::BothSynced(state) => Some(&state.calendar),
        };

        for local_change in &item.local_changes_to_apply {
            apply_change(provider.local(), initial_calendar_id, &item.id, local_change, false).await;
        }

        for remote_change in &item.remote_changes_to_apply {
            apply_change(provider.remote(), initial_calendar_id, &item.id, remote_change, true).await;
        }
    }

    provider
}

async fn get_or_insert_calendar<S, C>(source: &mut S, id: &CalendarId) -> Result<Arc<Mutex<C>>, Box<dyn Error>>
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match source.get_calendar(id).await {
        Some(cal) => Ok(cal),
        None => {
            let new_name = format!("Calendar for ID {}", id);
            let supported_components = SupportedComponents::TODO;
            let cal = C::new(new_name.to_string(), id.clone(), supported_components);
            source.insert_calendar(cal).await
        }
    }
}

/// Apply a single change on a given source
async fn apply_change<S, C>(source: &S, calendar_id: Option<&CalendarId>, item_id: &ItemId, change: &ChangeToApply, is_remote: bool)
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match calendar_id {
        Some(cal) => apply_changes_on_an_existing_item(source, cal, item_id, change, is_remote).await,
        None => create_test_item(source, change).await,
    }
}

async fn apply_changes_on_an_existing_item<S, C>(source: &S, calendar_id: &CalendarId, item_id: &ItemId, change: &ChangeToApply, is_remote: bool)
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    let cal = source.get_calendar(calendar_id).await.unwrap();
    let mut cal = cal.lock().unwrap();
    let task = cal.get_item_by_id_mut(item_id).await.unwrap().unwrap_task_mut();

    match change {
        ChangeToApply::Rename(new_name) => {
            if is_remote {
                task.mock_remote_calendar_set_name(new_name.clone());
            } else {
                task.set_name(new_name.clone());
            }
        },
        ChangeToApply::SetCompletion(new_status) => {
            if is_remote {
                task.mock_remote_calendar_set_completed(new_status.clone());
            } else {
                task.set_completed(new_status.clone());
            }
        },
        ChangeToApply::ChangeCalendar(_) => {
            panic!("Not implemented yet");
        },
        ChangeToApply::Remove => {
            match is_remote {
                false => cal.mark_for_deletion(item_id).await.unwrap(),
                true => cal.delete_item(item_id).await.unwrap(),
            };
        },
        ChangeToApply::Create(_calendar_id, _item) => {
            panic!("This function only handles already existing items");
        },
    }
}

async fn create_test_item<S, C>(source: &S, change: &ChangeToApply)
where
    S: CalDavSource<C>,
    C: CompleteCalendar + DavCalendar, // in this test, we're using a calendar that mocks both kinds
{
    match change {
        ChangeToApply::Rename(_) |
        ChangeToApply::SetCompletion(_) |
        ChangeToApply::ChangeCalendar(_) |
        ChangeToApply::Remove => {
            panic!("This function only creates items that do not exist yet");
        }
        ChangeToApply::Create(calendar_id, item) => {
            let cal = source.get_calendar(calendar_id).await.unwrap();
            cal.lock().unwrap().add_item(item.clone()).await.unwrap();
        },
    }
}
