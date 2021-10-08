/// A counter of errors that happen during a sync
pub struct SyncResult {
    n_errors: u32,
}
impl SyncResult {
    pub fn new() -> Self {
        Self { n_errors: 0 }
    }
    pub fn is_success(&self) -> bool {
        self.n_errors == 0
    }

    pub fn error(&mut self, text: &str) {
        log::error!("{}", text);
        self.n_errors += 1;
    }
    pub fn warn(&mut self, text: &str) {
        log::warn!("{}", text);
        self.n_errors += 1;
    }
    pub fn info(&mut self, text: &str) {
        log::info!("{}", text);
    }
    pub fn debug(&mut self, text: &str) {
        log::debug!("{}", text);
    }
    pub fn trace(&mut self, text: &str) {
        log::trace!("{}", text);
    }
}
