use super::{fixture, oracle, present, TestResult};
use crate::milesplit::{RetainedJournalBundle, OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use census_store::StoreError;
use serde_json::{json, Value};

const STAGES: [&str; 5] = [
    "capture_phase_started",
    "capture_phase_completed",
    "meet_phase_started",
    "meet_phase_completed",
    "counter_completed",
];

#[test]
fn observed_empty_and_chunked_reports_equal_quiet_and_independent_bundle_bytes() -> TestResult {
    let (_directory, store) = fixture()?;
    for populated in [false, true] {
        let entries = if populated {
            let body = json!({"body": "x".repeat(1_100_000), "escaped": "\n\\é"});
            store.journal_done(OWNED_CAPTURE_PHASE, "row/a-null", &Value::Null)?;
            store.journal_done(OWNED_MEET_PHASE, "row/z-\"\\é", &body)?;
            vec![
                present(OWNED_CAPTURE_PHASE, "row/a-null", Value::Null),
                present(OWNED_MEET_PHASE, "row/z-\"\\é", body),
            ]
        } else {
            Vec::new()
        };
        let held = store.snapshot();
        let expected = oracle(&held, entries)?;
        let mut stages = Vec::new();
        let observed = RetainedJournalBundle::measure_present_with_progress(&held, |stage| {
            stages.push(stage);
            Ok(())
        })?;
        let value = serde_json::to_value(&observed)?;
        let bytes = serde_json::to_vec(&observed)?;
        drop(observed);
        check!(eq; stages, STAGES);
        check!(eq; value, expected);
        check!(eq; bytes, serde_json::to_vec(&RetainedJournalBundle::measure_present(&held)?)?);
    }
    Ok(())
}

#[test]
fn mutation_between_phases_keeps_original_snapshot_for_all_metrics_and_digest() -> TestResult {
    let (_directory, store) = fixture()?;
    store.journal_done(OWNED_MEET_PHASE, "row/meet", &json!("old"))?;
    let held = store.snapshot();
    let before = serde_json::to_vec(&RetainedJournalBundle::measure_present(&held)?)?;
    let mut stages = Vec::new();
    let report = RetainedJournalBundle::measure_present_with_progress(&held, |stage| {
        stages.push(stage);
        if stage == "capture_phase_completed" {
            store.journal_done(OWNED_MEET_PHASE, "row/meet", &json!("new"))?;
        }
        Ok(())
    })?;
    let bytes = serde_json::to_vec(&report)?;
    drop(report);
    check!(eq; stages, STAGES);
    check!(eq; bytes, before);
    check!(
        serde_json::to_vec(&RetainedJournalBundle::measure_present(&store.snapshot())?)? != before
    );
    Ok(())
}

#[test]
fn every_observer_refusal_returns_exact_sentinel_and_stops_at_that_call() -> TestResult {
    let (_directory, store) = fixture()?;
    let held = store.snapshot();
    for (index, refused) in STAGES.iter().enumerate() {
        let mut attempted = Vec::new();
        let error = RetainedJournalBundle::measure_present_with_progress(&held, |stage| {
            attempted.push(stage);
            if stage == *refused {
                Err(StoreError::BudgetExceeded {
                    what: "progress sentinel",
                    needed: 19,
                    available: 7,
                })
            } else {
                Ok(())
            }
        })
        .err()
        .ok_or("observer error swallowed")?;
        check!(matches!(
            error,
            StoreError::BudgetExceeded {
                what: "progress sentinel",
                needed: 19,
                available: 7,
            }
        ));
        check!(eq; attempted, STAGES.iter().take(index.saturating_add(1)).copied().collect::<Vec<_>>());
    }
    Ok(())
}

#[test]
fn actual_capture_and_meet_refusals_never_emit_phase_completion() -> TestResult {
    for (phase, oversized_payload) in [
        (OWNED_CAPTURE_PHASE, false),
        (OWNED_MEET_PHASE, false),
        (OWNED_CAPTURE_PHASE, true),
        (OWNED_MEET_PHASE, true),
    ] {
        let (_directory, store) = fixture()?;
        store.journal_done(OWNED_CAPTURE_PHASE, "row/a", &Value::Null)?;
        let key = if oversized_payload {
            "row/z".into()
        } else {
            "k".repeat(513)
        };
        let value = if oversized_payload {
            json!("x".repeat(4_194_304))
        } else {
            Value::Null
        };
        store.journal_done(phase, &key, &value)?;
        let held = store.snapshot();
        let mut stages = Vec::new();
        let error = RetainedJournalBundle::measure_present_with_progress(&held, |stage| {
            stages.push(stage);
            Ok(())
        })
        .err()
        .ok_or("actual phase refusal suppressed")?;
        if oversized_payload {
            check!(
                matches!(error, StoreError::JournalReadLimit { phase: actual, key: actual_key,
                what: "encoded envelope", needed, max: 4_194_304 }
                if actual == phase && actual_key == key && needed > 4_194_304)
            );
        } else {
            check!(matches!(error, StoreError::Refused { detail }
                if detail == "unsupported retained source journal key"));
        }
        let prefix = if phase == OWNED_CAPTURE_PHASE { 1 } else { 3 };
        check!(eq; stages, STAGES.iter().take(prefix).copied().collect::<Vec<_>>());
        check!(eq; store.snapshot().sequence(), held.sequence());
    }
    Ok(())
}
