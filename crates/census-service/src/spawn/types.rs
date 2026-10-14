use crate::admission::DepartureReason;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskReport {
    pub accepted: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub aborted: u64,
    pub panicked: u64,
    pub failed: u64,
    pub still_running: u64,
    pub timed_out: u64,
    pub remaining: u64,
}

impl TaskReport {
    pub fn disjoint_total(&self) -> u64 {
        self.completed
            .saturating_add(self.cancelled)
            .saturating_add(self.aborted)
            .saturating_add(self.panicked)
            .saturating_add(self.still_running)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("starting an owned worker thread failed: {source}")]
    ThreadSpawn {
        #[source]
        source: std::io::Error,
    },
    #[error("owned worker thread accounting allocation failed")]
    ThreadAllocation,
    #[error("task descriptor accounting is inconsistent")]
    TaskCountMismatch,
    #[error("task count does not fit u64")]
    TaskCountOverflow,
    #[error("the task region already supervises its {capacity} tasks")]
    RegionFull { capacity: usize },
    #[error("the task region's capacity is closed")]
    RegionClosed,
    #[error("admission refused: {0}")]
    Admission(DepartureReason),
}

impl From<DepartureReason> for SpawnError {
    fn from(reason: DepartureReason) -> Self {
        match reason {
            DepartureReason::RegionClosed => Self::RegionClosed,
            reason => Self::Admission(reason),
        }
    }
}
