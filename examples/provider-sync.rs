use std::path::Path;

use my_tasks::{client::Client, traits::CalDavSource};
use my_tasks::calendar::{CalendarId, cached_calendar::CachedCalendar, remote_calendar::RemoteCalendar};
use my_tasks::Item;
use my_tasks::Task;
use my_tasks::cache::Cache;
use my_tasks::Provider;
use my_tasks::traits::BaseCalendar;
use my_tasks::settings::URL;
use my_tasks::settings::USERNAME;
use my_tasks::settings::PASSWORD;
use my_tasks::settings::EXAMPLE_CALENDAR_URL;
use my_tasks::utils::pause;

const CACHE_FOLDER: &str = "example_cache";


#[tokio::main]
async fn main() {
    env_logger::init();

    println!("This examples show how to sync a remote server with a local cache, using a Provider.");
    println!("Make sure you have edited your settings.rs to include correct URLs and credentials.");
    println!("You can also set the RUST_LOG environment variable to display more info about the sync.");
    pause();

    let cache_path = Path::new(CACHE_FOLDER);

    let client = Client::new(URL, USERNAME, PASSWORD).unwrap();
    let cache = match Cache::from_folder(&cache_path) {
        Ok(cache) => cache,
        Err(err) => {
            log::warn!("Invalid cache file: {}. Using a default cache", err);
            Cache::new(&cache_path)
        }
    };
    let mut provider = Provider::new(client, cache);

    let cals = provider.local().get_calendars().await.unwrap();
    println!("---- before sync -----");
    my_tasks::utils::print_calendar_list(&cals).await;

    println!("Starting a sync...");
    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync.");
    }
    provider.local().save_to_folder().unwrap();

    println!("---- after sync -----");
    let cals = provider.local().get_calendars().await.unwrap();
    my_tasks::utils::print_calendar_list(&cals).await;

    add_items_and_sync_again(&mut provider).await;
}

async fn add_items_and_sync_again(provider: &mut Provider<Cache, CachedCalendar, Client, RemoteCalendar>) {
    println!("Now, we'll add a task and run the sync again.");
    pause();

    let changed_calendar_id: CalendarId = EXAMPLE_CALENDAR_URL.parse().unwrap();
    let changed_calendar = provider.local().get_calendar(&changed_calendar_id).await.unwrap();

    let new_name = "New example task";
    let new_task = Task::new(String::from(new_name), false, &changed_calendar_id);
    changed_calendar.lock().unwrap().add_item(Item::Task(new_task)).await.unwrap();

    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync. The new task may not have been synced.");
    } else {
        println!("Done syncing the new task '{}'", new_name);
    }
    provider.local().save_to_folder().unwrap();

    println!("Done. You can start this example again to see the cache being restored from its current saved state")
}
