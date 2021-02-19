//! This module is the data source of the Caldav items
//!
//! This gives access to data from both the server and a local database for quick retrieval when the app starts

use std::sync::Arc;
use std::error::Error;

mod calendar;
mod task;
mod client;

pub use calendar::Calendar;
pub use task::Task;
use client::Client;

/// A Caldav data source
pub struct DataSource {
    client: Option<Client>,

    calendars: Vec<Calendar>
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
    pub fn set_server(&mut self, url: String, username: String, password: String) -> Result<(), Box<dyn Error>> {
        self.client = Some(Client::new(url, username, password)?);
        Ok(())
    }

    /// Update the local database with info from the Client
    pub fn fetch_from_server(&mut self) {

    }

    pub fn update_changes_to_server(&self) {

    }

    pub fn calendars(&self) -> Vec<&Calendar> {
        self.calendars
            .iter()
            .collect()
    }
}
