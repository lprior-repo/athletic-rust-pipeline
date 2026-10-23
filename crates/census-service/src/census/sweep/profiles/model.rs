use super::super::access;
use census_domain::model::AcquisitionDigest;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cursor {
    pub packet: AcquisitionDigest,
    pub member: usize,
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub packet: AcquisitionDigest,
    pub athlete_id: String,
    pub next: Cursor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Selection {
    Profile(Target),
    Advance { next: Cursor, gaps: usize },
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureOutcome {
    Captured,
    NotFound,
    SourceFailure {
        message: String,
        refusal: access::Refusal,
        retryable: bool,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Progress {
    pub(crate) cursor: Option<Cursor>,
    pub(crate) captured: u64,
    pub(crate) not_found: u64,
    pub(crate) failed: u64,
    pub(crate) source_gaps: u64,
    pub(crate) exhausted: bool,
    pub(crate) stopped: bool,
    pub(crate) awaiting_rosters: bool,
    pub(crate) scan_rounds: u64,
    pub(crate) last_failure: Option<String>,
    roster_witness: Option<AcquisitionDigest>,
    refusals: access::RefusalRun,
}

impl Progress {
    pub(crate) fn bind_rosters(
        &mut self,
        witness: AcquisitionDigest,
    ) -> census_store::StoreResult<()> {
        if self.roster_witness.as_ref() != Some(&witness) {
            let rounds = self
                .scan_rounds
                .checked_add(1)
                .ok_or(census_store::StoreError::CounterOverflow)?;
            self.cursor = None;
            self.exhausted = false;
            self.awaiting_rosters = false;
            self.roster_witness = Some(witness);
            self.scan_rounds = rounds;
        }
        Ok(())
    }

    pub(crate) fn finish_scan(&mut self, rosters_terminal: bool) {
        self.exhausted = rosters_terminal;
        self.awaiting_rosters = !rosters_terminal;
    }

    pub(crate) fn record(
        &mut self,
        target: Target,
        outcome: CaptureOutcome,
    ) -> census_store::StoreResult<()> {
        let (count, refusal) = match outcome {
            CaptureOutcome::Captured => (&mut self.captured, access::Refusal::None),
            CaptureOutcome::NotFound => (&mut self.not_found, access::Refusal::None),
            CaptureOutcome::SourceFailure {
                message, refusal, ..
            } => {
                self.last_failure = Some(message);
                (&mut self.failed, refusal)
            }
        };
        *count = count
            .checked_add(1)
            .ok_or(census_store::StoreError::CounterOverflow)?;
        self.stopped = self.refusals.observe(refusal);
        self.cursor = Some(target.next);
        Ok(())
    }

    pub(crate) fn advance(&mut self, next: Cursor, gaps: usize) -> census_store::StoreResult<()> {
        self.source_gaps = self
            .source_gaps
            .checked_add(
                u64::try_from(gaps).map_err(|_| census_store::StoreError::CounterOverflow)?,
            )
            .ok_or(census_store::StoreError::CounterOverflow)?;
        self.cursor = Some(next);
        Ok(())
    }
}
