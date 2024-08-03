mod scenarii;

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "local_calendar_mocks_remote_calendars")]
use kitchen_fridge::mock_behaviour::MockBehaviour;

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
    pub fn normal() -> Self {
        Self {}
    }
    pub fn first_sync_to_local() -> Self {
        Self {}
    }
    pub fn first_sync_to_server() -> Self {
        Self {}
    }
    pub fn transient_task() -> Self {
        Self {}
    }
    pub fn normal_with_errors1() -> Self {
        Self {}
    }
    pub fn normal_with_errors2() -> Self {
        Self {}
    }
    pub fn normal_with_errors3() -> Self {
        Self {}
    }
    pub fn normal_with_errors4() -> Self {
        Self {}
    }
    pub fn normal_with_errors5() -> Self {
        Self {}
    }
    pub fn normal_with_errors6() -> Self {
        Self {}
    }
    pub fn normal_with_errors7() -> Self {
        Self {}
    }
    pub fn normal_with_errors8() -> Self {
        Self {}
    }
    pub fn normal_with_errors9() -> Self {
        Self {}
    }
    pub fn normal_with_errors10() -> Self {
        Self {}
    }
    pub fn normal_with_errors11() -> Self {
        Self {}
    }
    pub fn normal_with_errors12() -> Self {
        Self {}
    }

    pub async fn run(&self, _max_attempts: u32) {
        panic!("WARNING: This test required the \"integration_tests\" Cargo feature");
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

    pub fn normal_with_errors1() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour::fail_now(10))),
        }
    }

    pub fn normal_with_errors2() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                get_calendars_behaviour: (0, 1),
                create_calendar_behaviour: (2, 2),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors3() -> Self {
        Self {
            scenarii: scenarii::scenarii_first_sync_to_server(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                get_calendars_behaviour: (1, 6),
                create_calendar_behaviour: (0, 1),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors4() -> Self {
        Self {
            scenarii: scenarii::scenarii_first_sync_to_server(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                add_item_behaviour: (1, 3),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors5() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                get_item_version_tags_behaviour: (0, 1),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors6() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                get_item_by_url_behaviour: (3, 2),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors7() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                delete_item_behaviour: (0, 2),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors8() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                add_item_behaviour: (2, 3),
                get_item_by_url_behaviour: (1, 12),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors9() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                get_calendars_behaviour: (0, 8),
                delete_item_behaviour: (1, 1),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors10() -> Self {
        Self {
            scenarii: scenarii::scenarii_first_sync_to_server(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                get_calendars_behaviour: (0, 8),
                delete_item_behaviour: (1, 1),
                create_calendar_behaviour: (1, 4),
                get_item_version_tags_behaviour: (3, 1),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors11() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                get_calendars_behaviour: (0, 8),
                delete_item_behaviour: (1, 1),
                create_calendar_behaviour: (1, 4),
                get_item_version_tags_behaviour: (3, 1),
                get_item_by_url_behaviour: (0, 41),
                ..MockBehaviour::default()
            })),
        }
    }

    pub fn normal_with_errors12() -> Self {
        Self {
            scenarii: scenarii::scenarii_basic(),
            mock_behaviour: Arc::new(Mutex::new(MockBehaviour {
                update_item_behaviour: (0, 3),
                ..MockBehaviour::default()
            })),
        }
    }

    pub async fn run(&self, max_attempts: u32) {
        self.mock_behaviour.lock().unwrap().suspend();

        let mut provider = scenarii::populate_test_provider_before_sync(
            &self.scenarii,
            Arc::clone(&self.mock_behaviour),
        )
        .await;
        print_provider(&provider, "before sync").await;

        self.mock_behaviour.lock().unwrap().resume();
        for attempt in 0..max_attempts {
            println!("\nSyncing...\n");
            if provider.sync().await == true {
                println!("Sync complete after {} attempts (multiple attempts are due to forced errors in mocked behaviour)", attempt+1);
                break;
            }
        }
        self.mock_behaviour.lock().unwrap().suspend();

        print_provider(&provider, "after sync").await;

        // Check the contents of both sources are the same after sync
        assert!(provider
            .remote()
            .has_same_observable_content_as(provider.local())
            .await
            .unwrap());

        // But also explicitely check that every item is expected
        let expected_provider = scenarii::populate_test_provider_after_sync(
            &self.scenarii,
            Arc::clone(&self.mock_behaviour),
        )
        .await;

        assert!(provider
            .local()
            .has_same_observable_content_as(expected_provider.local())
            .await
            .unwrap());
        assert!(provider
            .remote()
            .has_same_observable_content_as(expected_provider.remote())
            .await
            .unwrap());

        // Perform a second sync, even if no change has happened, just to check
        println!("Syncing again");
        provider.sync().await;
        assert!(provider
            .local()
            .has_same_observable_content_as(expected_provider.local())
            .await
            .unwrap());
        assert!(provider
            .remote()
            .has_same_observable_content_as(expected_provider.remote())
            .await
            .unwrap());
    }
}

async fn run_flavour(flavour: TestFlavour, max_attempts: u32) {
    let _ = env_logger::builder().is_test(true).try_init();
    flavour.run(max_attempts).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_regular_sync() {
    run_flavour(TestFlavour::normal(), 1).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_sync_empty_initial_local() {
    run_flavour(TestFlavour::first_sync_to_local(), 1).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_sync_empty_initial_server() {
    run_flavour(TestFlavour::first_sync_to_server(), 1).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_sync_transient_task() {
    run_flavour(TestFlavour::transient_task(), 1).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync1() {
    run_flavour(TestFlavour::normal_with_errors1(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync2() {
    run_flavour(TestFlavour::normal_with_errors2(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync3() {
    run_flavour(TestFlavour::normal_with_errors3(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync4() {
    run_flavour(TestFlavour::normal_with_errors4(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync5() {
    run_flavour(TestFlavour::normal_with_errors5(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync6() {
    run_flavour(TestFlavour::normal_with_errors6(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync7() {
    run_flavour(TestFlavour::normal_with_errors7(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync8() {
    run_flavour(TestFlavour::normal_with_errors8(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync9() {
    run_flavour(TestFlavour::normal_with_errors9(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync10() {
    run_flavour(TestFlavour::normal_with_errors10(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync11() {
    run_flavour(TestFlavour::normal_with_errors11(), 100).await;
}

#[tokio::test]
#[cfg_attr(not(feature = "integration_tests"), ignore)]
async fn test_errors_in_regular_sync12() {
    run_flavour(TestFlavour::normal_with_errors12(), 100).await;
}

#[cfg(feature = "integration_tests")]
use kitchen_fridge::{
    cache::Cache, calendar::cached_calendar::CachedCalendar, provider::Provider,
    traits::CalDavSource,
};

/// Print the contents of the provider. This is usually used for debugging
#[allow(dead_code)]
#[cfg(feature = "integration_tests")]
async fn print_provider(
    provider: &Provider<Cache, CachedCalendar, Cache, CachedCalendar>,
    title: &str,
) {
    let cals_server = provider.remote().get_calendars().await.unwrap();
    println!("----Server, {}-------", title);
    kitchen_fridge::utils::print_calendar_list(&cals_server).await;
    let cals_local = provider.local().get_calendars().await.unwrap();
    println!("-----Local, {}-------", title);
    kitchen_fridge::utils::print_calendar_list(&cals_local).await;
}
