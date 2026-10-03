use super::super::{boundary, recovery};
use super::StoreJournalRef;
use anyhow::Result;
use census_service::restate_services::{TeamsAttemptProgress, TeamsSourceInspection};

pub(in super::super) fn source_ledger(
    key: &str,
    inspection: &TeamsSourceInspection,
) -> Result<Vec<StoreJournalRef>> {
    let progress = recovery::progress(inspection);
    boundary::validate_progress(progress)?;
    let reference = |suffix: &str| StoreJournalRef {
        phase: "teams_source_attempts_v1",
        key: format!("{key}/{suffix}"),
    };
    let mut references = Vec::new();
    references.try_reserve_exact(7)?;
    if !progress.is_empty() {
        references.push(reference("identity"));
    }
    progress.iter().try_for_each(|step| -> Result<()> {
        let attempt = match step {
            TeamsAttemptProgress::Unknown { attempt }
            | TeamsAttemptProgress::Completed { attempt, .. }
            | TeamsAttemptProgress::Transient { attempt, .. }
            | TeamsAttemptProgress::Terminal { attempt, .. } => attempt,
        };
        references.push(reference(&format!("attempt/{attempt}/reserved")));
        if !matches!(step, TeamsAttemptProgress::Unknown { .. }) {
            references.push(reference(&format!("attempt/{attempt}/outcome")));
        }
        Ok(())
    })?;
    Ok(references)
}
