use std::path::Path;

use my_tasks::{client::Client, traits::CalDavSource};
use my_tasks::cache::Cache;
use my_tasks::Provider;
use my_tasks::settings::URL;
use my_tasks::settings::USERNAME;
use my_tasks::settings::PASSWORD;

const CACHE_FOLDER: &str = "example_cache";


#[tokio::main]
async fn main() {
    env_logger::init();

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

    if let Err(err) = provider.sync().await {
        log::error!("Unable to sync: {}", err);
    }
    println!("---- after sync -----");
    let cals = provider.local().get_calendars().await.unwrap();
    my_tasks::utils::print_calendar_list(&cals).await;
}
