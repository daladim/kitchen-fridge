//! This is an example of how kitchen-fridge can be used

use chrono::Utc;
use url::Url;

use kitchen_fridge::calendar::SupportedComponents;
use kitchen_fridge::task::CompletionStatus;
use kitchen_fridge::traits::BaseCalendar;
use kitchen_fridge::traits::CalDavSource;
use kitchen_fridge::traits::CompleteCalendar;
use kitchen_fridge::utils::pause;
use kitchen_fridge::CalDavProvider;
use kitchen_fridge::Item;
use kitchen_fridge::Task;

mod shared;
use shared::initial_sync;
use shared::{EXAMPLE_CREATED_CALENDAR_URL, EXAMPLE_EXISTING_CALENDAR_URL, URL, USERNAME};

const CACHE_FOLDER: &str = "test_cache/provider_sync";

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("This example show how to sync a remote server with a local cache, using a Provider.");
    println!("Make sure you have edited the constants in the 'shared.rs' file to include correct URLs and credentials.");
    println!(
        "You can also set the RUST_LOG environment variable to display more info about the sync."
    );
    println!("");
    println!("This will use the following settings:");
    println!("  * URL = {}", URL);
    println!("  * USERNAME = {}", USERNAME);
    println!(
        "  * EXAMPLE_EXISTING_CALENDAR_URL = {}",
        EXAMPLE_EXISTING_CALENDAR_URL
    );
    println!(
        "  * EXAMPLE_CREATED_CALENDAR_URL = {}",
        EXAMPLE_CREATED_CALENDAR_URL
    );
    pause();

    let mut provider = initial_sync(CACHE_FOLDER).await;

    add_items_and_sync_again(&mut provider).await;
}

async fn add_items_and_sync_again(provider: &mut CalDavProvider) {
    println!("\nNow, we'll add a calendar and a few tasks and run the sync again.");
    pause();

    // Create a new calendar...
    let new_calendar_url: Url = EXAMPLE_CREATED_CALENDAR_URL.parse().unwrap();
    let new_calendar_name = "A brave new calendar".to_string();
    if let Err(_err) = provider
        .local_mut()
        .create_calendar(
            new_calendar_url.clone(),
            new_calendar_name.clone(),
            SupportedComponents::TODO,
            Some("#ff8000".parse().unwrap()),
        )
        .await
    {
        println!("Unable to add calendar, maybe it exists already. We're not adding it after all.");
    }

    // ...and add a task in it
    let new_name = "This is a new task in a new calendar";
    let new_task = Task::new(String::from(new_name), true, &new_calendar_url);
    provider
        .local()
        .get_calendar(&new_calendar_url)
        .await
        .unwrap()
        .lock()
        .unwrap()
        .add_item(Item::Task(new_task))
        .await
        .unwrap();

    // Also create a task in a previously existing calendar
    let changed_calendar_url: Url = EXAMPLE_EXISTING_CALENDAR_URL.parse().unwrap();
    let new_task_name = "This is a new task we're adding as an example, with ÜTF-8 characters";
    let new_task = Task::new(String::from(new_task_name), false, &changed_calendar_url);
    let new_url = new_task.url().clone();
    provider
        .local()
        .get_calendar(&changed_calendar_url)
        .await
        .unwrap()
        .lock()
        .unwrap()
        .add_item(Item::Task(new_task))
        .await
        .unwrap();

    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync. The new task may not have been synced.");
    } else {
        println!(
            "Done syncing the new task '{}' and the new calendar '{}'",
            new_task_name, new_calendar_name
        );
    }
    provider.local().save_to_folder().unwrap();

    complete_item_and_sync_again(provider, &changed_calendar_url, &new_url).await;
}

async fn complete_item_and_sync_again(
    provider: &mut CalDavProvider,
    changed_calendar_url: &Url,
    url_to_complete: &Url,
) {
    println!("\nNow, we'll mark this last task as completed, and run the sync again.");
    pause();

    let completion_status = CompletionStatus::Completed(Some(Utc::now()));
    provider
        .local()
        .get_calendar(changed_calendar_url)
        .await
        .unwrap()
        .lock()
        .unwrap()
        .get_item_by_url_mut(url_to_complete)
        .await
        .unwrap()
        .unwrap_task_mut()
        .set_completion_status(completion_status);

    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync. The new task may not have been synced.");
    } else {
        println!("Done syncing the completed task");
    }
    provider.local().save_to_folder().unwrap();

    remove_items_and_sync_again(provider, changed_calendar_url, url_to_complete).await;
}

async fn remove_items_and_sync_again(
    provider: &mut CalDavProvider,
    changed_calendar_url: &Url,
    id_to_remove: &Url,
) {
    println!("\nNow, we'll delete this last task, and run the sync again.");
    pause();

    // Remove the task we had created
    provider
        .local()
        .get_calendar(changed_calendar_url)
        .await
        .unwrap()
        .lock()
        .unwrap()
        .mark_for_deletion(id_to_remove)
        .await
        .unwrap();

    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync. The new task may not have been synced.");
    } else {
        println!("Done syncing the deleted task");
    }
    provider.local().save_to_folder().unwrap();

    println!("Done. You can start this example again to see the cache being restored from its current saved state")
}
