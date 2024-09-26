use std::error::Error;

use reqwest::StatusCode;
use url::Url;

use crate::{calendar::remote_calendar::RemoteCalendarError, ical::IcalParseError, item::ItemType};

#[derive(Clone, Debug)]
pub enum HttpStatusConstraint {
    Success,

    /// It was required for the status to be one of those provided
    Specific(Vec<StatusCode>),
}

impl HttpStatusConstraint {
    pub fn satisfied_by(&self, status: StatusCode) -> bool {
        match self {
            Self::Success => status.is_success(),
            Self::Specific(statuses) => statuses.iter().any(|s| *s == status),
        }
    }

    pub fn assert(&self, status: StatusCode) -> Result<(), Box<dyn Error>> {
        if self.satisfied_by(status) {
            Ok(())
        } else {
            Err(KFError::UnexpectedHTTPStatusCode {
                expected: self.clone(),
                got: status,
            }
            .into())
        }
    }
}

/// Errors common to the Kitchen Fridge library
#[derive(thiserror::Error, Debug)]
pub enum KFError {
    #[error(
        "Calendar at URL {0} didn't appear in the client cache after being created on the server"
    )]
    CalendarDidNotSyncAfterCreation(Url),

    #[error("Error parsing '{text}': {source}")]
    DOMParseError {
        /// The text being parsed
        text: String,

        source: minidom::Error,
    },

    #[error("HTTP request {method} {url} resulted in an error: {source}")]
    HttpRequestError {
        url: Url,
        method: http::Method,
        source: reqwest::Error,
    },

    #[error("Error parsing ical data: {0}")]
    IcalParseError(#[from] IcalParseError),

    #[error("{detail}; an IO error occurred: {source}")]
    IoError {
        detail: String,
        source: std::io::Error,
    },

    #[error("{detail}; {type_:?} {url:?} already exists")]
    ItemAlreadyExists {
        type_: ItemType,
        detail: String,
        url: Url,
    },

    /// An item does not exist when it ought to have.
    ///
    /// type_ is None when the type of the item is unknown
    #[error("{detail}; {type_:?} {url:?} does not exist")]
    ItemDoesNotExist {
        type_: Option<ItemType>,
        detail: String,
        url: Url,
    },

    #[error("Missing DOM element {el} in {text}")]
    MissingDOMElement {
        /// The text that should have contained the element
        text: String,
        /// The element
        el: String,
    },

    #[error("An error occurred while mocking behavior: {0}")]
    #[cfg(feature = "local_calendar_mocks_remote_calendars")]
    MockError(#[from] crate::mock_behaviour::MockError),

    #[error("Remote calendar error: {0}")]
    RemoteCalendarError(#[from] RemoteCalendarError),

    #[error("Unexpected HTTP status code {got:?} but expected {expected:?}")]
    UnexpectedHTTPStatusCode {
        expected: HttpStatusConstraint,
        got: StatusCode,
    },
}

pub type KFResult<T> = Result<T, KFError>;
