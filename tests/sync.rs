use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{Utc, TimeZone};
use url::Url;

use my_tasks::traits::{CalDavSource, SyncSlave};
use my_tasks::traits::PartialCalendar;
use my_tasks::cache::Cache;
use my_tasks::Item;
use my_tasks::Task;
use my_tasks::calendar::cached_calendar::CachedCalendar;
use my_tasks::Provider;

#[tokio::test]
/// This test simulates a synchronisation between a local cache and a server
/// To "mock" a server, let's use a second cache
async fn test_regular_sync() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut provider = populate_test_provider().await;
    provider.sync().await.unwrap();


    let cals_server = provider.server().get_calendars().await.unwrap();
    println!("----Server-------");
    my_tasks::utils::print_calendar_list(&cals_server).await;
    let cals_local = provider.local().get_calendars().await.unwrap();
    println!("\n----Local-------");
    my_tasks::utils::print_calendar_list(&cals_local).await;

    assert!(provider.server().has_same_contents_than(provider.local()).await.unwrap());

}

/// Populate sources with the following:
/// * At the last sync: both sources had A, B, C, D, E, F, G, H, I, J, K, L, M at last sync
/// * Before the newer sync, this will be the content of the sources:
///     * server: A,    C, D,  E', F',  G✓, H , I',      K✓,    M, N
///     * cache:  A, B,    D', E,  F'', G , H✓, I✓, J✓,        M,    O
///
/// Hence, here is the expected result after the sync:
///     * both:   A,       D', E', F',  G✓, H✓, I',      K✓,   M, N, O
///
/// Notes:
/// * X': name has been modified since the last sync
/// * F'/F'': name conflict
/// * G✓: task has been marked as completed
async fn populate_test_provider() -> Provider<Cache, CachedCalendar, Cache, CachedCalendar> {
    let mut server = Cache::new(&PathBuf::from(String::from("server.json")));
    let mut local = Cache::new(&PathBuf::from(String::from("local.json")));

    let cal_id = Url::parse("http://todo.list/cal").unwrap();

    let task_a = Item::Task(Task::new("task A".into(), Utc.ymd(2000, 1, 1).and_hms(0, 0, 0)));
    let task_b = Item::Task(Task::new("task B".into(), Utc.ymd(2000, 1, 2).and_hms(0, 0, 0)));
    let task_c = Item::Task(Task::new("task C".into(), Utc.ymd(2000, 1, 3).and_hms(0, 0, 0)));
    let task_d = Item::Task(Task::new("task D".into(), Utc.ymd(2000, 1, 4).and_hms(0, 0, 0)));
    let task_e = Item::Task(Task::new("task E".into(), Utc.ymd(2000, 1, 5).and_hms(0, 0, 0)));
    let task_f = Item::Task(Task::new("task F".into(), Utc.ymd(2000, 1, 6).and_hms(0, 0, 0)));
    let task_g = Item::Task(Task::new("task G".into(), Utc.ymd(2000, 1, 7).and_hms(0, 0, 0)));
    let task_h = Item::Task(Task::new("task H".into(), Utc.ymd(2000, 1, 8).and_hms(0, 0, 0)));
    let task_i = Item::Task(Task::new("task I".into(), Utc.ymd(2000, 1, 9).and_hms(0, 0, 0)));
    let task_j = Item::Task(Task::new("task J".into(), Utc.ymd(2000, 1, 10).and_hms(0, 0, 0)));
    let task_k = Item::Task(Task::new("task K".into(), Utc.ymd(2000, 1, 11).and_hms(0, 0, 0)));
    let task_l = Item::Task(Task::new("task L".into(), Utc.ymd(2000, 1, 12).and_hms(0, 0, 0)));
    let task_m = Item::Task(Task::new("task M".into(), Utc.ymd(2000, 1, 12).and_hms(0, 0, 0)));

    let last_sync = task_m.last_modified();
    local.update_last_sync(Some(last_sync));
    assert!(last_sync < Utc::now());

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
    calendar.add_item(task_a);
    calendar.add_item(task_b);
    calendar.add_item(task_c);
    calendar.add_item(task_d);
    calendar.add_item(task_e);
    calendar.add_item(task_f);
    calendar.add_item(task_g);
    calendar.add_item(task_h);
    calendar.add_item(task_i);
    calendar.add_item(task_j);
    calendar.add_item(task_k);
    calendar.add_item(task_l);
    calendar.add_item(task_m);

    server.add_calendar(Arc::new(Mutex::new(calendar.clone())));
    local.add_calendar(Arc::new(Mutex::new(calendar.clone())));

    // Step 2
    // Edit the server calendar
    let cal_server = server.get_calendar(cal_id.clone()).await.unwrap();
    let mut cal_server = cal_server.lock().unwrap();

    cal_server.delete_item(&task_b_id).unwrap();

    cal_server.get_item_by_id_mut(&task_e_id).unwrap().unwrap_task_mut()
        .set_name("E has been remotely renamed".into());

    cal_server.get_item_by_id_mut(&task_f_id).unwrap().unwrap_task_mut()
        .set_name("F renamed in the server".into());

    cal_server.get_item_by_id_mut(&task_g_id).unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_server.get_item_by_id_mut(&task_i_id).unwrap().unwrap_task_mut()
        .set_name("I renamed in the server".into());

    cal_server.delete_item(&task_j_id).unwrap();

    cal_server.get_item_by_id_mut(&task_k_id).unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_server.delete_item(&task_l_id).unwrap();

    let task_n = Item::Task(Task::new("task N (new from server)".into(), Utc::now()));
    cal_server.add_item(task_n);


    // Step 3
    // Edit the local calendar
    let cal_local = local.get_calendar(cal_id).await.unwrap();
    let mut cal_local = cal_local.lock().unwrap();

    cal_local.delete_item(&task_c_id).unwrap();

    cal_local.get_item_by_id_mut(&task_d_id).unwrap().unwrap_task_mut()
        .set_name("D has been locally renamed".into());

    cal_local.get_item_by_id_mut(&task_f_id).unwrap().unwrap_task_mut()
        .set_name("F renamed locally as well!".into());

    cal_local.get_item_by_id_mut(&task_h_id).unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_local.get_item_by_id_mut(&task_i_id).unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_local.get_item_by_id_mut(&task_j_id).unwrap().unwrap_task_mut()
        .set_completed(true);

    cal_local.delete_item(&task_k_id).unwrap();
    cal_local.delete_item(&task_l_id).unwrap();

    let task_o = Item::Task(Task::new("task O (new from local)".into(), Utc::now()));
    cal_local.add_item(task_o);

    Provider::new(server, local)
}
