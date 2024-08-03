//! This is an example of how kitchen-fridge can be used.
//! This binary simply toggles all completion statuses of the tasks it finds.

use std::error::Error;

use chrono::Utc;

use kitchen_fridge::item::Item;
use kitchen_fridge::task::CompletionStatus;
use kitchen_fridge::utils::pause;
use kitchen_fridge::CalDavProvider;

mod shared;
use shared::initial_sync;
use shared::{URL, USERNAME};

const CACHE_FOLDER: &str = "test_cache/toggle_completion";

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
    pause();

    let mut provider = initial_sync(CACHE_FOLDER).await;

    toggle_all_tasks_and_sync_again(&mut provider)
        .await
        .unwrap();
}

async fn toggle_all_tasks_and_sync_again(
    provider: &mut CalDavProvider,
) -> Result<(), Box<dyn Error>> {
    let mut n_toggled = 0;

    for (_url, cal) in provider.local().get_calendars_sync()?.iter() {
        for (_url, item) in cal.lock().unwrap().get_items_mut_sync()?.iter_mut() {
            match item {
                Item::Task(task) => {
                    match task.completed() {
                        false => task
                            .set_completion_status(CompletionStatus::Completed(Some(Utc::now()))),
                        true => task.set_completion_status(CompletionStatus::Uncompleted),
                    };
                    n_toggled += 1;
                }
                Item::Event(_) => {
                    // Not doing anything with calendar events
                }
            }
        }
    }

    println!("{} items toggled.", n_toggled);
    println!("Syncing...");

    provider.sync().await;

    println!("Syncing complete.");

    Ok(())
}
