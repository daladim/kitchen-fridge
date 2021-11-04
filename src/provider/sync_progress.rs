//! Utilities to track the progression of a sync

use std::fmt::{Display, Error, Formatter};

/// An event that happens during a sync
#[derive(Clone, Debug)]
pub enum SyncEvent {
    /// Sync has not started
    NotStarted,
    /// Sync has just started but no calendar is handled yet
    Started,
    /// Sync is in progress.
    InProgress{ calendar: String, details: String},
    /// Sync is finished
    Finished{ success: bool },
}

impl Display for SyncEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            SyncEvent::NotStarted => write!(f, "Not started"),
            SyncEvent::Started => write!(f, "Sync has started..."),
            SyncEvent::InProgress{calendar, details} => write!(f, "[{}] {}...", calendar, details),
            SyncEvent::Finished{success} => match success {
                true => write!(f, "Sync successfully finished"),
                false => write!(f, "Sync finished with errors"),
            }
        }
    }
}

impl Default for SyncEvent {
    fn default() -> Self {
        Self::NotStarted
    }
}



/// See [`feedback_channel`]
pub type FeedbackSender = tokio::sync::watch::Sender<SyncEvent>;
/// See [`feedback_channel`]
pub type FeedbackReceiver = tokio::sync::watch::Receiver<SyncEvent>;

/// Create a feeback channel, that can be used to retrieve the current progress of a sync operation
pub fn feedback_channel() -> (FeedbackSender, FeedbackReceiver) {
    tokio::sync::watch::channel(SyncEvent::default())
}




/// A structure that tracks the progression and the errors that happen during a sync
pub struct SyncProgress {
    n_errors: u32,
    feedback_channel: Option<FeedbackSender>
}
impl SyncProgress {
    pub fn new() -> Self {
        Self { n_errors: 0, feedback_channel: None }
    }
    pub fn new_with_feedback_channel(channel: FeedbackSender) -> Self {
        Self { n_errors: 0, feedback_channel: Some(channel) }
    }


    pub fn is_success(&self) -> bool {
        self.n_errors == 0
    }

    /// Log an error
    pub fn error(&mut self, text: &str) {
        log::error!("{}", text);
        self.n_errors += 1;
    }
    /// Log a warning
    pub fn warn(&mut self, text: &str) {
        log::warn!("{}", text);
        self.n_errors += 1;
    }
    /// Log an info
    pub fn info(&mut self, text: &str) {
        log::info!("{}", text);
    }
    /// Log a debug message
    pub fn debug(&mut self, text: &str) {
        log::debug!("{}", text);
    }
    /// Log a trace message
    pub fn trace(&mut self, text: &str) {
        log::trace!("{}", text);
    }
    /// Send an event as a feedback to the listener (if any).
    pub fn feedback(&mut self, event: SyncEvent) {
        self.feedback_channel
            .as_ref()
            .map(|sender| {
                sender.send(event)
            });
    }
}
