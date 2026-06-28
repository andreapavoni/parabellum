//! Replay request and summary models.

use mini_cqrs_es::StoredEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayTarget {
    Village,
    Reports,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
    DryRun,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRequest {
    pub target: ReplayTarget,
    pub mode: ReplayMode,
    pub from_global_seq: i64,
    pub to_global_seq: Option<i64>,
    pub aggregate_id: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReplaySummary {
    pub scanned: usize,
    pub applied: usize,
    pub skipped: usize,
    pub first_global_seq: Option<i64>,
    pub last_global_seq: Option<i64>,
}

impl ReplaySummary {
    pub(super) fn record_scanned_event(&mut self, event: &StoredEvent) {
        self.scanned += 1;
        let Some(global_seq) = event.global_sequence else {
            return;
        };
        self.first_global_seq = Some(
            self.first_global_seq
                .map_or(global_seq, |current| current.min(global_seq)),
        );
        self.last_global_seq = Some(
            self.last_global_seq
                .map_or(global_seq, |current| current.max(global_seq)),
        );
    }
}
