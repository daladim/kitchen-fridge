use std::error::Error;

use async_trait::async_trait;

use crate::Calendar;

#[async_trait]
pub trait CalDavSource {
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars(&self) -> Result<&Vec<Calendar>, Box<dyn Error>>;

    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars_mut(&mut self) -> Result<Vec<&mut Calendar>, Box<dyn Error>>;
}
