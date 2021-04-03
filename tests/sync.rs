#![cfg(feature = "integration_tests")]
mod scenarii;

use my_tasks::traits::CalDavSource;
use my_tasks::Provider;
use my_tasks::cache::Cache;
use my_tasks::calendar::cached_calendar::CachedCalendar;


#[tokio::test]
/// This test simulates a regular synchronisation between a local cache and a server.
/// Note that this uses a second cache to "mock" a server.
async fn test_regular_sync() {
    let _ = env_logger::builder().is_test(true).try_init();

    let scenarii = scenarii::basic_scenarii();
    let mut provider = scenarii::populate_test_provider_before_sync(&scenarii).await;

    print_provider(&provider, "before sync").await;

    println!("\nsyncing...\n");
    provider.sync().await.unwrap();

    print_provider(&provider, "after sync").await;

    // Check the contents of both sources are the same after sync
    assert!(provider.remote().has_same_contents_than(provider.local()).await.unwrap());

    // But also explicitely check that every item is expected
    let expected_provider = scenarii::populate_test_provider_after_sync(&scenarii).await;
    println!("\n");
    print_provider(&expected_provider, "expected after sync").await;

    assert!(provider.local() .has_same_contents_than(expected_provider.local() ).await.unwrap());
    assert!(provider.remote().has_same_contents_than(expected_provider.remote()).await.unwrap());
}

/// Print the contents of the provider. This is usually used for debugging
#[allow(dead_code)]
async fn print_provider(provider: &Provider<Cache, CachedCalendar, Cache, CachedCalendar>, title: &str) {
    let cals_server = provider.remote().get_calendars().await.unwrap();
    println!("----Server, {}-------", title);
    my_tasks::utils::print_calendar_list(&cals_server).await;
    let cals_local = provider.local().get_calendars().await.unwrap();
    println!("-----Local, {}-------", title);
    my_tasks::utils::print_calendar_list(&cals_local).await;
}
