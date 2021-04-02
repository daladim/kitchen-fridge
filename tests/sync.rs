#![cfg(feature = "integration_tests")]
mod scenarii;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use url::Url;

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


#[tokio::test]
/// This test simulates a regular synchronisation between a local cache and a server.
/// Note that this uses a second cache to "mock" a server.
async fn test_regular_sync() {
    let scenarii = scenarii::basic_scenarii();
    let provider = scenarii::populate_test_provider(&scenarii).await;
}


#[tokio::test]
/// This test simulates a synchronisation between a local cache and a server
/// To "mock" a server, let's use a second cache
async fn legacy_test() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut provider = populate_test_provider().await;
    let cals_server = provider.remote().get_calendars().await.unwrap();
    println!("----Server, before sync-------");
    my_tasks::utils::print_calendar_list(&cals_server).await;
    let cals_local = provider.local().get_calendars().await.unwrap();
    println!("\n----Local, before sync-------");
    my_tasks::utils::print_calendar_list(&cals_local).await;

    provider.sync().await.unwrap();


    let cals_server = provider.remote().get_calendars().await.unwrap();
    println!("----Server, after sync-------");
    my_tasks::utils::print_calendar_list(&cals_server).await;
    let cals_local = provider.local().get_calendars().await.unwrap();
    println!("\n----Local, after sync-------");
    my_tasks::utils::print_calendar_list(&cals_local).await;

    assert!(provider.remote().has_same_contents_than(provider.local()).await.unwrap());

}


async fn populate_test_provider() -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    let mut server = Cache::new(&PathBuf::from(String::from("server.json")));
    let mut local = Cache::new(&PathBuf::from(String::from("local.json")));

    let cal_id = Url::parse("http://todo.list/cal").unwrap();

    let task_a = Item::Task(Task::new("task A".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_b = Item::Task(Task::new("task B".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_c = Item::Task(Task::new("task C".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_d = Item::Task(Task::new("task D".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_e = Item::Task(Task::new("task E".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_f = Item::Task(Task::new("task F".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_g = Item::Task(Task::new("task G".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_h = Item::Task(Task::new("task H".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_i = Item::Task(Task::new("task I".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_j = Item::Task(Task::new("task J".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_k = Item::Task(Task::new("task K".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_l = Item::Task(Task::new("task L".into(), ItemId::random(), SyncStatus::random_synced(), false));
    let task_m = Item::Task(Task::new("task M".into(), ItemId::random(), SyncStatus::random_synced(), false));

    let task_b_id = task_b.id().clone();
    let task_c_id = task_c.id().clone();
    let task_d_id = task_d.id().clone();
    let task_e_id = task_e.id().clone();
    let task_f_id = task_f.id().clone();
    let task_g_id = task_g.id().clone();
    let task_h_id = task_h.id().clone();
    let task_i_id = task_i.id().clone();
    let task_j_id = task_j.id().clone();
    let task_k_id = task_k.id().clone();
    let task_l_id = task_l.id().clone();

    // Step 1
    // Build the calendar as it was at the time of the sync
    let mut calendar = CachedCalendar::new("a list".into(), cal_id.clone(), my_tasks::calendar::SupportedComponents::TODO);
    calendar.add_item(task_a).await.unwrap();
    calendar.add_item(task_b).await.unwrap();
    calendar.add_item(task_c).await.unwrap();
    calendar.add_item(task_d).await.unwrap();
    calendar.add_item(task_e).await.unwrap();
    calendar.add_item(task_f).await.unwrap();
    calendar.add_item(task_g).await.unwrap();
    calendar.add_item(task_h).await.unwrap();
    calendar.add_item(task_i).await.unwrap();
    calendar.add_item(task_j).await.unwrap();
    calendar.add_item(task_k).await.unwrap();
    calendar.add_item(task_l).await.unwrap();
    calendar.add_item(task_m).await.unwrap();

    server.insert_calendar(calendar.clone());
    local.insert_calendar(calendar.clone());

    // Step 2
    // Edit the server calendar
    let cal_server = server.get_calendar(&cal_id).await.unwrap();
    let mut cal_server = cal_server.lock().unwrap();

    cal_server.delete_item(&task_b_id).await.unwrap();

    cal_server.get_item_by_id_mut(&task_e_id).await.unwrap().unwrap_task_mut()
        .mock_remote_calendar_set_name("E has been remotely renamed".into());

    cal_server.get_item_by_id_mut(&task_f_id).await.unwrap().unwrap_task_mut()
        .mock_remote_calendar_set_name("F renamed in the server".into());

    cal_server.get_item_by_id_mut(&task_g_id).await.unwrap().unwrap_task_mut()
        .mock_remote_calendar_set_completed(true);

    cal_server.get_item_by_id_mut(&task_i_id).await.unwrap().unwrap_task_mut()
        .mock_remote_calendar_set_name("I renamed in the server".into());

    cal_server.delete_item(&task_j_id).await.unwrap();

    cal_server.get_item_by_id_mut(&task_k_id).await.unwrap().unwrap_task_mut()
        .mock_remote_calendar_set_completed(true);

    cal_server.delete_item(&task_l_id).await.unwrap();

    let task_n = Item::Task(Task::new("task N (new from server)".into(), ItemId::random(), SyncStatus::random_synced(), false));
    cal_server.add_item(task_n).await.unwrap();


    // Step 3
    // Edit the local calendar
    let cal_local = local.get_calendar(&cal_id).await.unwrap();
    let mut cal_local = cal_local.lock().unwrap();

    cal_local.mark_for_deletion(&task_c_id).await.unwrap();

    cal_local.get_item_by_id_mut(&task_d_id).await.unwrap().unwrap_task_mut()
        .set_name("D has been locally renamed".into());

    cal_local.get_item_by_id_mut(&task_f_id).await.unwrap().unwrap_task_mut()
        .set_name("F renamed locally as well!".into());

    cal_local.get_item_by_id_mut(&task_h_id).await.unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_local.get_item_by_id_mut(&task_i_id).await.unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_local.get_item_by_id_mut(&task_j_id).await.unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_local.mark_for_deletion(&task_k_id).await.unwrap();
    cal_local.mark_for_deletion(&task_l_id).await.unwrap();

    let task_o = Item::Task(Task::new("task O (new from local)".into(), ItemId::random(), SyncStatus::NotSynced, false));
    cal_local.add_item(task_o).await.unwrap();

    Provider::new(server, local)
}
