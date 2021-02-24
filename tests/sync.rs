use std::path::PathBuf;

use chrono::{Utc, TimeZone};
use url::Url;

use my_tasks::traits::CalDavSource;
use my_tasks::cache::Cache;
use my_tasks::Task;
use my_tasks::Calendar;
use my_tasks::Provider;

#[tokio::test]
/// This test simulates a synchronisation between a local cache and a server
/// To "mock" a server, let's use a second cache
async fn test_sync() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut provider = populate_test_provider().await;
    provider.sync().await.unwrap();

    let cal_server = provider.server().get_calendars().await.unwrap();
    let cal_local = provider.local().get_calendars().await.unwrap();
    assert_eq!(cal_server, cal_local, "{:#?}\n{:#?}", cal_server, cal_local);

    panic!("TODO: also check that the contents are expected!");
}

/// Populate sources with the following:
/// * At the last sync: both sources had A, B, C, D, E, F, G, H at last sync
/// * Before the newer sync, this will be the content of the sources:
///     * server: A,    C, D,  E', F',  G~, H , I
///     * cache:  A, B,    D', E,  F'', G , H~,   J
///
/// Notes:
/// * X': name has been modified since the last sync
/// * F'/F'': name conflict
/// * G~: task has been marked as completed
async fn populate_test_provider() -> Provider<Cache, Cache> {
    let mut server = Cache::new(&PathBuf::from(String::from("server.json")));
    let mut local = Cache::new(&PathBuf::from(String::from("local.json")));

    let task_a = Task::new("task A".into(), Utc.ymd(2000, 1, 1).and_hms(0, 0, 0));
    let task_b = Task::new("task B".into(), Utc.ymd(2000, 1, 2).and_hms(0, 0, 0));
    let task_c = Task::new("task C".into(), Utc.ymd(2000, 1, 3).and_hms(0, 0, 0));
    let task_d = Task::new("task D".into(), Utc.ymd(2000, 1, 4).and_hms(0, 0, 0));
    let task_e = Task::new("task E".into(), Utc.ymd(2000, 1, 5).and_hms(0, 0, 0));
    let task_f = Task::new("task F".into(), Utc.ymd(2000, 1, 6).and_hms(0, 0, 0));
    let task_g = Task::new("task G".into(), Utc.ymd(2000, 1, 7).and_hms(0, 0, 0));
    let task_h = Task::new("task H".into(), Utc.ymd(2000, 1, 8).and_hms(0, 0, 0));

    let last_sync = task_h.last_modified();
    assert!(last_sync < Utc::now());

    let task_b_id = task_b.id().clone();
    let task_c_id = task_c.id().clone();
    let task_d_id = task_d.id().clone();
    let task_e_id = task_e.id().clone();
    let task_f_id = task_f.id().clone();
    let task_g_id = task_g.id().clone();
    let task_h_id = task_h.id().clone();

    // Step 1
    // Build the calendar as it was at the time of the sync
    let mut calendar = Calendar::new("a list".into(), Url::parse("http://todo.list/cal").unwrap(), my_tasks::calendar::SupportedComponents::TODO);
    calendar.add_task(task_a);
    calendar.add_task(task_b);
    calendar.add_task(task_c);
    calendar.add_task(task_d);
    calendar.add_task(task_e);
    calendar.add_task(task_f);
    calendar.add_task(task_g);
    calendar.add_task(task_h);

    server.add_calendar(calendar.clone());
    local.add_calendar(calendar.clone());

    // Step 2
    // Edit the server calendar
    let cal_server = &mut server.get_calendars_mut().await.unwrap()[0];

    cal_server.delete_task(&task_b_id);

    cal_server.get_task_by_id_mut(&task_e_id).unwrap()
        .set_name("E has been remotely renamed".into());

    cal_server.get_task_by_id_mut(&task_f_id).unwrap()
        .set_name("F renamed in the server".into());

    cal_server.get_task_by_id_mut(&task_g_id).unwrap()
        .set_completed(true);

    let task_i = Task::new("task I".into(), Utc::now());
    cal_server.add_task(task_i);


    // Step 3
    // Edit the local calendar
    let cal_local = &mut local.get_calendars_mut().await.unwrap()[0];

    cal_local.delete_task(&task_c_id);

    cal_local.get_task_by_id_mut(&task_d_id).unwrap()
        .set_name("D has been locally renamed".into());

    cal_local.get_task_by_id_mut(&task_f_id).unwrap()
        .set_name("F renamed locally as well!".into());

    cal_local.get_task_by_id_mut(&task_h_id).unwrap()
        .set_completed(true);

    let task_j = Task::new("task J".into(), Utc::now());
    cal_local.add_task(task_j);

    Provider::new(server, local, last_sync)
}
