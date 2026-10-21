use super::refusal;
use crate::restate_services::wire::{
    JurisdictionRequest, JurisdictionState, StageOutcome, TeamsAttemptProgress, TeamsFailure,
    TeamsSourceOutcome, TeamsStage,
};
use restate_sdk::prelude::*;
use std::io::{self, Write};

const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_ROWS: usize = 65_536;
const MAX_SOURCES: usize = 21;

pub(super) struct Budget {
    bytes: usize,
    rows: usize,
}

impl Budget {
    pub(super) fn new() -> Self {
        Self { bytes: 0, rows: 0 }
    }

    pub(super) fn value(
        &mut self,
        value: &(impl serde::Serialize + ?Sized),
    ) -> Result<(), HandlerError> {
        serde_json::to_writer(self, value).map_err(|_| refusal("serialized input budget exhausted"))
    }

    pub(super) fn parent(
        &mut self,
        state: &JurisdictionState,
        request: &JurisdictionRequest,
    ) -> Result<(), HandlerError> {
        self.value(state)?;
        self.value(request)?;
        if let Some(plan) = &state.plan {
            let count = plan
                .sweepable
                .len()
                .checked_add(plan.refused.len())
                .and_then(|value| value.checked_add(plan.satisfied.len()));
            if count.is_none_or(|count| count > MAX_SOURCES) {
                return Err(refusal("source inventory bound exceeded"));
            }
        }
        if let TeamsStage::Failed(TeamsFailure::SourceFailures {
            outcome, failures, ..
        }) = &state.teams
        {
            if failures.len() > MAX_SOURCES {
                return Err(refusal("source failure bound exceeded"));
            }
            self.stage(outcome)?;
            for failure in failures {
                self.progress(&failure.outcome)?;
            }
        }
        Ok(())
    }

    pub(super) fn source(&mut self, source: &TeamsSourceOutcome) -> Result<(), HandlerError> {
        self.value(source)?;
        self.progress(source)
    }

    fn progress(&mut self, source: &TeamsSourceOutcome) -> Result<(), HandlerError> {
        let progress = match source {
            TeamsSourceOutcome::Completed { outcome, progress } => {
                self.stage(outcome)?;
                progress
            }
            TeamsSourceOutcome::Terminal { progress, .. }
            | TeamsSourceOutcome::Interrupted { progress, .. }
            | TeamsSourceOutcome::Exhausted { progress, .. } => progress,
        };
        if progress.len() > 3 {
            return Err(refusal("source attempt history bound exceeded"));
        }
        for attempt in progress {
            match attempt {
                TeamsAttemptProgress::Completed { outcome, .. } => self.stage(outcome)?,
                TeamsAttemptProgress::Transient {
                    outcome: Some(outcome),
                    ..
                }
                | TeamsAttemptProgress::Terminal {
                    outcome: Some(outcome),
                    ..
                } => self.stage(outcome)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn stage(&mut self, stage: &StageOutcome) -> Result<(), HandlerError> {
        self.rows = [
            stage.errors.len(),
            stage.notes.len(),
            stage.unfinished.len(),
        ]
        .into_iter()
        .try_fold(self.rows, |sum, count| sum.checked_add(count))
        .filter(|rows| *rows <= MAX_ROWS)
        .ok_or_else(|| refusal("diagnostic row budget exhausted"))?;
        Ok(())
    }
}

impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .checked_add(bytes.len())
            .filter(|value| *value <= MAX_BYTES)
            .ok_or_else(|| io::Error::other("ordinary teams input capacity"))?;
        self.bytes = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
