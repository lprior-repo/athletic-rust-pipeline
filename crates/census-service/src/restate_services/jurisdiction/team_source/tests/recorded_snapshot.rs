use super::*;
use crate::restate_services::wire::TeamsSourceInspection;
use census_crawl::{CollectionDisposition, CrawlError};

fn secondary() -> CrawlError {
    CrawlError::Resource {
        resource: "source access snapshot",
        requested: 4097,
        limit: 4096,
    }
}

#[test]
fn completed_physical_result_is_withheld_but_retained_after_store_reopen() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let retained = {
        let store = Store::open(root.path())?;
        let (attempt, at) = reserve(&store, &request)?;
        let result = super::super::access_snapshot::failure(
            ledger::Attempt::Completed(StageOutcome {
                records: 17,
                at,
                errors: Vec::new(),
                notes: vec!["controlled acquired coaches".into()],
                disposition: CollectionDisposition::Complete,
                unfinished: Vec::new(),
            }),
            secondary(),
        );
        let ledger::Completion::Settled(TeamsSourceOutcome::Terminal { progress, .. }) =
            ledger::finish(&store, &operation, attempt, result, &[])?
        else {
            return Err("missing access evidence must not complete acquisition".into());
        };
        let [TeamsAttemptProgress::Terminal {
            attempt: 1,
            outcome: Some(outcome),
            ..
        }] = progress.as_slice()
        else {
            return Err("known physical progress was lost or became unknown".into());
        };
        check!(eq; outcome.records, 17);
        check!(eq; outcome.at, "2026-10-02");
        check!(eq; outcome.notes, vec!["controlled acquired coaches"]);
        check!(eq; outcome.disposition, CollectionDisposition::Partial);
        let [refusal] = outcome.errors.as_slice() else {
            return Err("access-evidence refusal was not retained".into());
        };
        check!(refusal.contains("source access snapshot"));
        let snapshot =
            serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?;
        store.flush()?;
        snapshot
    };
    let store = Store::open(root.path())?;
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?, retained);
    let state = ledger::durable_attempts(&store)?;
    check!(eq; (state.reserved, state.recorded, state.unknown, state.settled), (1, 1, 0, 1));
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    Ok(())
}

#[test]
fn incomplete_physical_result_retains_rejection_and_retry_class_after_reopen() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let retained = {
        let store = Store::open(root.path())?;
        let (attempt, at) = reserve(&store, &request)?;
        let result = super::super::access_snapshot::failure(
            ledger::Attempt::Incomplete {
                outcome: StageOutcome {
                    records: 8,
                    at,
                    errors: vec!["controlled original partial refusal".into()],
                    notes: vec!["controlled partial acquisition".into()],
                    disposition: CollectionDisposition::Blocked,
                    unfinished: vec!["https://fixtures.invalid/pending".into()],
                },
                error: JobError::Transient {
                    message: "controlled original HTTP 429".into(),
                },
            },
            secondary(),
        );
        let ledger::Completion::Retry(JobError::Transient { message }) =
            ledger::finish(&store, &operation, attempt, result, &[])?
        else {
            return Err("snapshot failure changed the original retry class".into());
        };
        check!(message.contains("controlled original HTTP 429"));
        check!(message.contains("source access snapshot"));
        let inspection = ledger::inspection(&store, &operation, Some(&request))?;
        let TeamsSourceInspection::Unsettled { progress } = &inspection else {
            return Err("incomplete acquisition was falsely settled".into());
        };
        let [TeamsAttemptProgress::Transient {
            attempt: 1,
            outcome: Some(outcome),
            ..
        }] = progress.as_slice()
        else {
            return Err("incomplete physical progress was lost".into());
        };
        check!(eq; outcome.records, 8);
        check!(eq; outcome.at, "2026-10-02");
        check!(eq; outcome.notes, vec!["controlled partial acquisition"]);
        check!(eq; outcome.unfinished, vec!["https://fixtures.invalid/pending"]);
        check!(eq; outcome.disposition, CollectionDisposition::Blocked);
        let [original, refusal] = outcome.errors.as_slice() else {
            return Err("original or access-evidence refusal was lost".into());
        };
        check!(eq; original, "controlled original partial refusal");
        check!(refusal.contains("source access snapshot"));
        let snapshot = serde_json::to_value(&inspection)?;
        store.flush()?;
        snapshot
    };
    let store = Store::open(root.path())?;
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?, retained);
    let state = ledger::durable_attempts(&store)?;
    check!(eq; (state.reserved, state.recorded, state.unknown, state.settled), (1, 1, 0, 0));
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    Ok(())
}
