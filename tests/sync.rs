mod scenarii;




/// A test that simulates a regular synchronisation between a local cache and a server.
/// Note that this uses a second cache to "mock" a server.
struct TestFlavour {
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    scenarii: Vec<scenarii::ItemScenario>,
}

impl TestFlavour {
    #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
    pub fn normal() -> Self { Self{} }
    #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
    pub fn first_sync_to_local() -> Self { Self{} }
    #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
    pub fn first_sync_to_server() -> Self { Self{} }

    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn normal() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
        }
    }

    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn first_sync_to_local() -> Self {
        Self {
            scenarii: scenarii::scenarii_first_sync_to_local(),
        }
    }

    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub fn first_sync_to_server() -> Self {
        Self {
            scenarii: scenarii::scenarii_first_sync_to_server(),
        }
    }


    #[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
    pub async fn run(&self) {
        println!("WARNING: This test required the \"integration_tests\" Cargo feature");
    }

    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    pub async fn run(&self) {
        let mut provider = scenarii::populate_test_provider_before_sync(&self.scenarii).await;

        print_provider(&provider, "before sync").await;

        println!("\nsyncing...\n");
        provider.sync().await.unwrap();

        print_provider(&provider, "after sync").await;

        // Check the contents of both sources are the same after sync
        assert!(provider.remote().has_same_observable_content_as(provider.local()).await.unwrap());

        // But also explicitely check that every item is expected
        let expected_provider = scenarii::populate_test_provider_after_sync(&self.scenarii).await;
        println!("\n");
        print_provider(&expected_provider, "expected after sync").await;

        assert!(provider.local() .has_same_observable_content_as(expected_provider.local() ).await.unwrap());
        assert!(provider.remote().has_same_observable_content_as(expected_provider.remote()).await.unwrap());
    }
}




#[tokio::test]
async fn test_regular_sync() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::normal();
    flavour.run().await;
}

#[tokio::test]
async fn test_sync_empty_initial_local() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::first_sync_to_local();
    flavour.run().await;
}

#[tokio::test]
async fn test_sync_empty_initial_server() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::first_sync_to_server();
    flavour.run().await;
}


#[cfg(feature = "integration_tests")]
use my_tasks::{traits::CalDavSource,
               Provider,
               cache::Cache,
               calendar::cached_calendar::CachedCalendar,
};

/// Print the contents of the provider. This is usually used for debugging
#[allow(dead_code)]
#[cfg(feature = "integration_tests")]
async fn print_provider(provider: &Provider<Cache, CachedCalendar, Cache, CachedCalendar>, title: &str) {
    let cals_server = provider.remote().get_calendars().await.unwrap();
    println!("----Server, {}-------", title);
    my_tasks::utils::print_calendar_list(&cals_server).await;
    let cals_local = provider.local().get_calendars().await.unwrap();
    println!("-----Local, {}-------", title);
    my_tasks::utils::print_calendar_list(&cals_local).await;
}
