use std::path::Path;

use chrono::{Utc};

use my_tasks::{client::Client, traits::CalDavSource};
use my_tasks::calendar::{CalendarId, SupportedComponents};
use my_tasks::calendar::cached_calendar::CachedCalendar;
use my_tasks::calendar::remote_calendar::RemoteCalendar;
use my_tasks::Item;
use my_tasks::Task;
use my_tasks::task::CompletionStatus;
use my_tasks::ItemId;
use my_tasks::cache::Cache;
use my_tasks::Provider;
use my_tasks::traits::BaseCalendar;
use my_tasks::traits::CompleteCalendar;
use my_tasks::settings::URL;
use my_tasks::settings::USERNAME;
use my_tasks::settings::PASSWORD;
use my_tasks::settings::EXAMPLE_CREATED_CALENDAR_URL;
use my_tasks::settings::EXAMPLE_EXISTING_CALENDAR_URL;
use my_tasks::utils::pause;

const CACHE_FOLDER: &str = "test_cache/provider_sync";


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
    println!("---- Local items, before sync -----");
    my_tasks::utils::print_calendar_list(&cals).await;

    println!("Starting a sync...");
    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync.");
    }
    provider.local().save_to_folder().unwrap();

    println!("---- Local items, after sync -----");
    let cals = provider.local().get_calendars().await.unwrap();
    my_tasks::utils::print_calendar_list(&cals).await;

    add_items_and_sync_again(&mut provider).await;
}

async fn add_items_and_sync_again(
    provider: &mut Provider<Cache, CachedCalendar, Client, RemoteCalendar>)
{
    println!("\nNow, we'll add a calendar and a few tasks and run the sync again.");
    pause();

    // Create a new calendar...
    let new_calendar_id: CalendarId = EXAMPLE_CREATED_CALENDAR_URL.parse().unwrap();
    let new_calendar_name = "A brave new calendar".to_string();
    if let Err(_err) = provider.local_mut()
        .create_calendar(new_calendar_id.clone(), new_calendar_name.clone(), SupportedComponents::TODO)
        .await {
            println!("Unable to add calendar, maybe it exists already. We're not adding it after all.");
    }

    // ...and add a task in it
    let new_name = "This is a new task in a new calendar";
    let new_task = Task::new(String::from(new_name), true, &new_calendar_id);
    provider.local().get_calendar(&new_calendar_id).await.unwrap()
        .lock().unwrap().add_item(Item::Task(new_task)).await.unwrap();


    // Also create a task in a previously existing calendar
    let changed_calendar_id: CalendarId = EXAMPLE_EXISTING_CALENDAR_URL.parse().unwrap();
    let new_task_name = "This is a new task we're adding as an example, with ÜTF-8 characters";
    let new_task = Task::new(String::from(new_task_name), false, &changed_calendar_id);
    let new_id = new_task.id().clone();
    provider.local().get_calendar(&changed_calendar_id).await.unwrap()
        .lock().unwrap().add_item(Item::Task(new_task)).await.unwrap();


    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync. The new task may not have been synced.");
    } else {
        println!("Done syncing the new task '{}' and the new calendar '{}'", new_task_name, new_calendar_name);
    }
    provider.local().save_to_folder().unwrap();

    complete_item_and_sync_again(provider, &changed_calendar_id, &new_id).await;
}

async fn complete_item_and_sync_again(
    provider: &mut Provider<Cache, CachedCalendar, Client, RemoteCalendar>,
    changed_calendar_id: &CalendarId,
    id_to_complete: &ItemId)
{
    println!("\nNow, we'll mark this last task as completed, and run the sync again.");
    pause();

    let completion_status = CompletionStatus::Completed(Some(Utc::now()));
    provider.local().get_calendar(changed_calendar_id).await.unwrap()
        .lock().unwrap().get_item_by_id_mut(id_to_complete).await.unwrap()
        .unwrap_task_mut()
        .set_completion_status(completion_status);

    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync. The new task may not have been synced.");
    } else {
        println!("Done syncing the completed task");
    }
    provider.local().save_to_folder().unwrap();

    remove_items_and_sync_again(provider, changed_calendar_id, id_to_complete).await;
}

async fn remove_items_and_sync_again(
    provider: &mut Provider<Cache, CachedCalendar, Client, RemoteCalendar>,
    changed_calendar_id: &CalendarId,
    id_to_remove: &ItemId)
{
    println!("\nNow, we'll delete this last task, and run the sync again.");
    pause();

    // Remove the task we had created
    provider.local().get_calendar(changed_calendar_id).await.unwrap()
        .lock().unwrap()
        .mark_for_deletion(id_to_remove).await.unwrap();

    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync. The new task may not have been synced.");
    } else {
        println!("Done syncing the deleted task");
    }
    provider.local().save_to_folder().unwrap();

    println!("Done. You can start this example again to see the cache being restored from its current saved state")
}
