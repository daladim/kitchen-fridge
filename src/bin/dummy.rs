use my_tasks::client::Client;
use my_tasks::settings::URL;
use my_tasks::settings::USERNAME;
use my_tasks::settings::PASSWORD;


#[tokio::main]
async fn main() {
    // This is just a function to silence "unused function" warning

    let mut client = Client::new(URL, USERNAME, PASSWORD).unwrap();
    let calendars = client.get_calendars().await.unwrap();
    let _ = calendars.iter()
        .map(|cal| println!("  {}\t{}", cal.name(), cal.url().as_str()))
        .collect::<()>();
    let _ = client.get_tasks(&calendars[3].url()).await;
}
