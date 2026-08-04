use std::time::Instant;

use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct RunSummary {
    pub environments_total: usize,
    pub environments_with_sites: usize,
    pub environments_no_power_pages_sites: usize,
    pub websites_total: usize,
    pub endpoints_attempted: usize,
    pub endpoints_http_error: usize,
    pub endpoints_no_scan: usize,
    pub hec_events_sent: usize,
    pub duration_secs: u64,
    pub preflight_ok: bool,
    #[serde(skip)]
    started_at: Option<Instant>,
}

impl RunSummary {
    pub fn new() -> Self {
        Self {
            started_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn record_preflight(&mut self, http_status: u16) {
        self.preflight_ok = !matches!(http_status, 401 | 403);
    }

    pub fn record_endpoint(&mut self, outcome: crate::CollectionOutcome, event_count: usize) {
        self.endpoints_attempted += 1;
        self.hec_events_sent += event_count;
        if outcome == crate::CollectionOutcome::HttpError {
            self.endpoints_http_error += 1;
        }
        if outcome == crate::CollectionOutcome::NoScan {
            self.endpoints_no_scan += 1;
        }
    }

    pub fn record_env_with_sites(&mut self, website_count: usize) {
        self.environments_with_sites += 1;
        self.websites_total += website_count;
    }

    pub fn record_env_no_sites(&mut self) {
        self.environments_no_power_pages_sites += 1;
    }

    pub fn finish(mut self) -> Self {
        if let Some(started) = self.started_at.take() {
            self.duration_secs = started.elapsed().as_secs();
        }
        self
    }
}
