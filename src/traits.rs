//! This module provides CalDAV data sources and utilities

/*
use std::error::Error;



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
        // TODO: how to handle conflicts?
    }

    pub fn update_changes_to_server(&self) {

    }

    // TODO: the API should force calling fetch_from_server before
    pub fn calendars(&self) -> Vec<&Calendar> {
        // TODO: what happens when a user has a reference, which is modified/updated from the server? Conflict mut/not mut?
        // TODO: how can the user modify Tasks from a non-mut reference?
        self.calendars
            .iter()
            .collect()
    }
}

*/
