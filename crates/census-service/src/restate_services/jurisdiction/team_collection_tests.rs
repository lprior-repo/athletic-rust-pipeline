use std::error::Error;

use super::team_collection::retain;
use crate::restate_services::wire::{StageOutcome, TeamsFailure, TeamsSourceOutcome, TeamsStage};

#[test]
fn exhausted_source_retains_failure_without_discarding_later_completed_source(
) -> Result<(), Box<dyn Error>> {
    let mut aggregate = StageOutcome {
        records: 0,
        at: "2026-10-02".to_string(),
        errors: Vec::new(),
        notes: Vec::new(),
        disposition: census_crawl::CollectionDisposition::Complete,
        unfinished: Vec::new(),
    };
    let mut failures = Vec::new();
    retain(
        "arbiter_orgs",
        TeamsSourceOutcome::Exhausted {
            attempts: 3,
            last_failure: "source interrupted after partial acquisition".to_string(),
            progress: Vec::new(),
        },
        &mut aggregate,
        &mut failures,
    )?;
    retain(
        "coach_directories",
        TeamsSourceOutcome::Completed {
            outcome: StageOutcome {
                records: 19,
                at: "2026-10-02".to_string(),
                errors: Vec::new(),
                notes: vec!["retained independently qualified contacts".to_string()],
                disposition: census_crawl::CollectionDisposition::Complete,
                unfinished: Vec::new(),
            },
            progress: Vec::new(),
        },
        &mut aggregate,
        &mut failures,
    )?;
    let state = TeamsStage::Failed(TeamsFailure::SourceFailures {
        at: "2026-10-02".to_string(),
        outcome: aggregate,
        failures,
    });
    let restored: TeamsStage = serde_json::from_value(serde_json::to_value(state)?)?;
    check!(!restored.is_owed());
    check!(!restored.is_completed());
    let TeamsStage::Failed(TeamsFailure::SourceFailures {
        outcome, failures, ..
    }) = restored
    else {
        return Err("source failures lost their typed state".into());
    };
    check!(eq; outcome.records, 19);
    check!(eq;
        outcome.notes,
        vec!["coach_directories: retained independently qualified contacts"]
    );
    let Some(failure) = failures.first() else {
        return Err("exhausted source lost".into());
    };
    check!(eq; failure.source, "arbiter_orgs");
    check!(matches!(
        failure.outcome,
        TeamsSourceOutcome::Exhausted { attempts: 3, .. }
    ));
    Ok(())
}

#[test]
fn source_aggregation_refuses_overflow_without_losing_existing_progress(
) -> Result<(), Box<dyn Error>> {
    let mut aggregate = StageOutcome {
        records: usize::MAX,
        at: "2026-10-02".to_string(),
        errors: Vec::new(),
        notes: Vec::new(),
        disposition: census_crawl::CollectionDisposition::Complete,
        unfinished: Vec::new(),
    };
    let mut failures = Vec::new();
    let result = retain(
        "wiaa",
        TeamsSourceOutcome::Completed {
            outcome: StageOutcome {
                records: 1,
                at: "2026-10-02".to_string(),
                errors: Vec::new(),
                notes: Vec::new(),
                disposition: census_crawl::CollectionDisposition::Complete,
                unfinished: Vec::new(),
            },
            progress: Vec::new(),
        },
        &mut aggregate,
        &mut failures,
    );
    check!(result.is_err());
    check!(eq; aggregate.records, usize::MAX);
    check!(eq; aggregate.notes, Vec::<String>::new());
    Ok(())
}

#[test]
fn exhaustion_keeps_last_durable_prefix_and_exact_unfinished_locator() -> Result<(), Box<dyn Error>>
{
    let prefix = StageOutcome {
        records: 7,
        at: "2026-10-02".to_string(),
        errors: vec!["retained source failure".to_string()],
        notes: Vec::new(),
        disposition: census_crawl::CollectionDisposition::Partial,
        unfinished: vec!["https://www.wiaawi.org/Schools?page=2".to_string()],
    };
    let mut aggregate = StageOutcome {
        records: 0,
        at: prefix.at.clone(),
        errors: Vec::new(),
        notes: Vec::new(),
        disposition: census_crawl::CollectionDisposition::Complete,
        unfinished: Vec::new(),
    };
    let mut failures = Vec::new();
    retain(
        "wiaa",
        TeamsSourceOutcome::Exhausted {
            attempts: 3,
            last_failure: "timeout".to_string(),
            progress: vec![
                crate::restate_services::wire::TeamsAttemptProgress::Transient {
                    attempt: 1,
                    outcome: Some(prefix.clone()),
                    message: "partial first pull".to_string(),
                },
                crate::restate_services::wire::TeamsAttemptProgress::Transient {
                    attempt: 2,
                    outcome: Some(prefix),
                    message: "cached replay".to_string(),
                },
                crate::restate_services::wire::TeamsAttemptProgress::Unknown { attempt: 3 },
            ],
        },
        &mut aggregate,
        &mut failures,
    )?;
    check!(eq; aggregate.records, 7);
    check!(eq; aggregate.unfinished, vec!["https://www.wiaawi.org/Schools?page=2"]);
    check!(eq; aggregate.disposition, census_crawl::CollectionDisposition::Partial);
    check!(eq; failures.len(), 1);
    check!(eq; aggregate.errors, vec!["wiaa: retained source failure", "wiaa: exhausted after 3 attempts: timeout"]);
    Ok(())
}
