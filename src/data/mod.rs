//! This module is the data source of the Caldav items
//!
//! This gives access to data from both the server and a local database for quick retrieval when the app starts

use std::sync::Arc;

mod calendar;
mod tasks;
mod client;

pub use calendar::Calendar;
pub use tasks::Task;
use client::Client;

// TODO: consider using references here
//       (there will be no issue with still-borrowed-data when the DataSource is destroyed, but will it play well with sync stuff?)
type CalendarView = Arc<Calendar>;
type TaskView = Arc<Task>;

/// A Caldav data source
pub struct DataSource {
    client: Option<Client>,

    calendars: Vec<CalendarView>
}

impl DataSource {
    /// Create a new data source
    pub fn new() -> Self {
        Self{
            client: None,
            calendars: Vec::new(),
        }
    }

    /// Tell this data source what the source server is
    pub fn set_server(&mut self, url: String, username: String, password: String) {
        self.client = Client::new(url, username, password);
    }

    /// Update the local database with info from the Client
    pub fn fetch_from_server(&self) {
        // TODO: how to handle conflicts?
    }

    pub fn calendars(&self) -> Vec<CalendarView> {
        self.calendars
    }
}

impl Drop for DataSource {
    fn drop(&mut self) {
        // TODO: display a warning in case some CalendarViews still have a refcount > 0
    }
}
