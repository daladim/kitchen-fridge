mod scenarii;

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use my_tasks::mock_behaviour::MockBehaviour;



/// A test that simulates a regular synchronisation between a local cache and a server.
/// Note that this uses a second cache to "mock" a server.
struct TestFlavour {
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    scenarii: Vec<scenarii::ItemScenario>,
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    mock_behaviour: Arc<Mutex<MockBehaviour>>,
}

#[cfg(not(feature = "local_calendar_mocks_remote_calendars"))]
impl TestFlavour {
    pub fn normal() -> Self { Self{} }
    pub fn first_sync_to_local() -> Self { Self{} }
    pub fn first_sync_to_server() -> Self { Self{} }
    pub fn transient_task() -> Self { Self{} }
    pub fn normal_with_errors() -> Self { Self{} }

    pub async fn run(&self, _max_attempts: u32) {
        println!("WARNING: This test required the \"integration_tests\" Cargo feature");
    }
}

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
impl TestFlavour {
    pub fn normal() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour::new())),
        }
    }

    pub fn first_sync_to_local() -> Self {
        Self {
            scenarii: scenarii::scenarii_first_sync_to_local(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour::new())),
        }
    }

    pub fn first_sync_to_server() -> Self {
        Self {
            scenarii: scenarii::scenarii_first_sync_to_server(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour::new())),
        }
    }

    pub fn transient_task() -> Self {
        Self {
            scenarii: scenarii::scenarii_transient_task(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour::new())),
        }
    }

    pub fn normal_with_errors() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour::fail_now(10))),
        }
    }


    pub async fn run(&self, max_attempts: u32) {
        self.mock_behaviour.lock().unwrap().suspend();

        let mut provider = scenarii::populate_test_provider_before_sync(&self.scenarii, Arc::clone(&self.mock_behaviour)).await;
        print_provider(&provider, "before sync").await;

        self.mock_behaviour.lock().unwrap().resume();
        for attempt in 0..max_attempts {
            println!("\nSyncing...\n");
            if provider.sync().await == true {
                println!("Sync complete after {} attempts (multiple attempts are due to forced errors in mocked behaviour)", attempt+1);
                break
            }
        }
        self.mock_behaviour.lock().unwrap().suspend();

        print_provider(&provider, "after sync").await;

        // Check the contents of both sources are the same after sync
        assert!(provider.remote().has_same_observable_content_as(provider.local()).await.unwrap());

        // But also explicitely check that every item is expected
        let expected_provider = scenarii::populate_test_provider_after_sync(&self.scenarii, Arc::clone(&self.mock_behaviour)).await;
        println!("\n");
        print_provider(&expected_provider, "expected after sync").await;

        assert!(provider.local() .has_same_observable_content_as(expected_provider.local() ).await.unwrap());
        assert!(provider.remote().has_same_observable_content_as(expected_provider.remote()).await.unwrap());

        // Perform a second sync, even if no change has happened, just to check
        println!("Syncing again");
        provider.sync().await;
        assert!(provider.local() .has_same_observable_content_as(expected_provider.local() ).await.unwrap());
        assert!(provider.remote().has_same_observable_content_as(expected_provider.remote()).await.unwrap());
    }
}




#[tokio::test]
async fn test_regular_sync() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::normal();
    flavour.run(1).await;
}

#[tokio::test]
async fn test_sync_empty_initial_local() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::first_sync_to_local();
    flavour.run(1).await;
}

#[tokio::test]
async fn test_sync_empty_initial_server() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::first_sync_to_server();
    flavour.run(1).await;
}

#[tokio::test]
async fn test_sync_transient_task() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::transient_task();
    flavour.run(1).await;
}

#[tokio::test]
async fn test_errors_in_regular_sync() {
    let _ = env_logger::builder().is_test(true).try_init();

    let flavour = TestFlavour::normal_with_errors();
    flavour.run(100).await;
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
