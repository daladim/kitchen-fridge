/*
use std::path::Path;

use my_tasks::{client::Client, traits::CalDavSource};
use my_tasks::cache::Cache;
use my_tasks::Provider;
use my_tasks::settings::URL;
use my_tasks::settings::USERNAME;
use my_tasks::settings::PASSWORD;

const CACHE_FILE: &str = "caldav_cache.json";
*/

#[tokio::main]
async fn main() {
    /*
    let cache_path = Path::new(CACHE_FILE);

    let mut client = Client::new(URL, USERNAME, PASSWORD).unwrap();
    let mut cache = match Cache::from_file(&cache_path) {
        Ok(cache) => cache,
        Err(err) => {
            log::warn!("Invalid cache file: {}. Using a default cache", err);
            Cache::new(&cache_path)
        }
    };
    let provider = Provider::new(client, cache);

    let cals = provider.local().get_calendars().await.unwrap();
    println!("---- before sync -----");
    my_tasks::utils::print_calendar_list(cals);

    provider.sync();
    println!("---- after sync -----");
    my_tasks::utils::print_calendar_list(cals);
    */

}
