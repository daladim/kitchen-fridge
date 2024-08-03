use std::path::Path;

use kitchen_fridge::cache::Cache;
use kitchen_fridge::client::Client;
use kitchen_fridge::traits::CalDavSource;
use kitchen_fridge::CalDavProvider;

// TODO: change these values with yours
pub const URL: &str = "https://my.server.com/remote.php/dav/files/john";
pub const USERNAME: &str = "username";
pub const PASSWORD: &str = "secret_password";

pub const EXAMPLE_EXISTING_CALENDAR_URL: &str =
    "https://my.server.com/remote.php/dav/calendars/john/a_calendar_name/";
pub const EXAMPLE_CREATED_CALENDAR_URL: &str =
    "https://my.server.com/remote.php/dav/calendars/john/a_calendar_that_we_have_created/";

fn main() {
    panic!("This file is not supposed to be executed");
}

/// Initializes a Provider, and run an initial sync from the server
pub async fn initial_sync(cache_folder: &str) -> CalDavProvider {
    let cache_path = Path::new(cache_folder);

    let client = Client::new(URL, USERNAME, PASSWORD).unwrap();
    let cache = match Cache::from_folder(&cache_path) {
        Ok(cache) => cache,
        Err(err) => {
            log::warn!("Invalid cache file: {}. Using a default cache", err);
            Cache::new(&cache_path)
        }
    };
    let mut provider = CalDavProvider::new(client, cache);

    let cals = provider.local().get_calendars().await.unwrap();
    println!("---- Local items, before sync -----");
    kitchen_fridge::utils::print_calendar_list(&cals).await;

    println!("Starting a sync...");
    println!(
        "Depending on your RUST_LOG value, you may see more or less details about the progress."
    );
    // Note that we could use sync_with_feedback() to have better and formatted feedback
    if provider.sync().await == false {
        log::warn!("Sync did not complete, see the previous log lines for more info. You can safely start a new sync.");
    }
    provider.local().save_to_folder().unwrap();

    println!("---- Local items, after sync -----");
    let cals = provider.local().get_calendars().await.unwrap();
    kitchen_fridge::utils::print_calendar_list(&cals).await;

    provider
}
