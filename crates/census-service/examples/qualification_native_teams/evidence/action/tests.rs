use super::*;
use crate::qualification_native_teams::http::Observation;
use crate::qualification_native_teams::scenario;
use std::collections::BTreeMap;

fn exhausted() -> Result<(SourceRun, Snapshot, JurisdictionRequest)> {
    let (parent, body) = scenario::request()?;
    let request: JurisdictionRequest = serde_json::from_value(body)?;
    let key = format!("{parent}/teams/milesplit");
    let progress = (1..=3).map(|attempt| serde_json::from_value(
        json!({"status":"transient", "attempt":attempt, "outcome":null, "message":format!("actual TLS EOF {attempt}")})))
        .collect::<serde_json::Result<Vec<_>>>()?;
    let outcome = TeamsSourceOutcome::Exhausted {
        attempts: 3,
        last_failure: "actual TLS EOF 3".to_string(),
        progress,
    };
    let body = serde_json::to_value(&outcome)?;
    let observation = Observation {
        method: "GET".into(),
        url: "http://127.0.0.1/".into(),
        sent_at: "observed".into(),
        received_at: "observed".into(),
        status: 200,
        error_source: None,
        invocation_id: Some("source1".into()),
        body,
    };
    let source = SourceRun {
        source: "milesplit".into(),
        key: key.clone(),
        id: "source1".into(),
        invocation: json!({"status":"completed", "completion_result":"success"}),
        output: observation.clone(),
        shared_state: observation,
        outcome: Some(outcome),
        journal: json!([]),
        events: json!([]),
    };
    let mut entries = BTreeMap::new();
    let digest = serialized_digest(&(
        request.jurisdiction,
        request.season,
        request.revision,
        &source.source,
    ))?;
    entries.insert(
        format!("{key}/identity"),
        json!({"request_digest":digest, "observed_on":"2026-10-02"}),
    );
    (1..=3).for_each(|attempt| {
        entries.insert(
            format!("{key}/attempt/{attempt}/reserved"),
            json!({"attempt":attempt, "observed_on":"2026-10-02"}),
        );
        entries.insert(
            format!("{key}/attempt/{attempt}/outcome"),
            json!({"status":"transient", "message":format!("actual TLS EOF {attempt}")}),
        );
    });
    entries.insert(format!("{key}/settled"), source.shared_state.body.clone());
    Ok((
        source,
        Snapshot {
            phase: super::super::super::ledger::PHASE,
            sequence: 0,
            entries,
        },
        request,
    ))
}

#[test]
fn qualification_counts_three_durable_slots_not_native_retry_metadata() -> Result<()> {
    let (mut source, snapshot, request) = exhausted()?;
    source.invocation["retry_count"] = json!(91);
    let actual = history(&source, &snapshot, &request, "2026-10-02")?;
    assert_eq!(actual.get("reservations"), Some(&json!([1, 2, 3])));
    assert_eq!(actual.get("recorded_outcomes"), Some(&json!([1, 2, 3])));
    assert_eq!(actual.get("consistent"), Some(&json!(true)));
    Ok(())
}

#[test]
fn qualification_refuses_exhaustion_when_any_reserved_or_outcome_slot_is_missing() -> Result<()> {
    (1..=3)
        .flat_map(|attempt| ["reserved", "outcome"].map(move |suffix| (attempt, suffix)))
        .try_for_each(|(attempt, suffix)| -> Result<()> {
            let (source, mut snapshot, request) = exhausted()?;
            snapshot
                .entries
                .remove(&format!("{}/attempt/{attempt}/{suffix}", source.key));
            let actual = history(&source, &snapshot, &request, "2026-10-02")?;
            assert_eq!(
                actual.get("consistent"),
                Some(&json!(false)),
                "{attempt}/{suffix}"
            );
            Ok(())
        })
}

#[test]
fn qualification_refuses_a_fourth_reservation_and_unknown_attempt_keys() -> Result<()> {
    [
        "attempt/4/reserved",
        "attempt/4/outcome",
        "attempt/03/reserved",
        "attempt/2/unknown",
    ]
    .iter()
    .try_for_each(|suffix| -> Result<()> {
        let (source, mut snapshot, request) = exhausted()?;
        snapshot
            .entries
            .insert(format!("{}/{suffix}", source.key), json!({"attempt":4}));
        let actual = history(&source, &snapshot, &request, "2026-10-02")?;
        assert_eq!(actual.get("consistent"), Some(&json!(false)), "{suffix}");
        assert_eq!(
            actual.get("legal_keys_no_attempt_four_or_unknown_suffix"),
            Some(&json!(false))
        );
        Ok(())
    })
}

#[test]
fn qualification_refuses_mismatched_identity_reservation_dates_and_last_failure() -> Result<()> {
    [
        (
            "identity",
            json!({"request_digest":"different", "observed_on":"2026-10-02"}),
        ),
        (
            "attempt/2/reserved",
            json!({"attempt":2, "observed_on":"2026-10-03"}),
        ),
        (
            "attempt/2/reserved",
            json!({"attempt":1, "observed_on":"2026-10-02"}),
        ),
        (
            "attempt/3/outcome",
            json!({"status":"transient", "message":"different failure"}),
        ),
    ]
    .iter()
    .try_for_each(|(suffix, value)| -> Result<()> {
        let (source, mut snapshot, request) = exhausted()?;
        snapshot
            .entries
            .insert(format!("{}/{suffix}", source.key), value.clone());
        let actual = history(&source, &snapshot, &request, "2026-10-02")?;
        assert_eq!(actual.get("consistent"), Some(&json!(false)), "{suffix}");
        Ok(())
    })
}

#[test]
fn qualification_distinguishes_unknown_interruption_from_consumed_final_reservation() -> Result<()>
{
    [None, Some(1), Some(3)]
        .iter()
        .try_for_each(|attempts| -> Result<()> {
            let (mut source, mut snapshot, request) = exhausted()?;
            snapshot
                .entries
                .remove(&format!("{}/attempt/3/outcome", source.key));
            let progress = serde_json::from_value(json!([
                {"status":"transient", "attempt":1, "outcome":null, "message":"actual TLS EOF 1"},
                {"status":"transient", "attempt":2, "outcome":null, "message":"actual TLS EOF 2"},
                {"status":"unknown", "attempt":3}
            ]))?;
            source.outcome = Some(TeamsSourceOutcome::Interrupted {
                attempts: *attempts,
                message: "interrupted".into(),
                progress,
            });
            source.shared_state.body = serde_json::to_value(&source.outcome)?;
            snapshot.entries.insert(
                format!("{}/settled", source.key),
                source.shared_state.body.clone(),
            );
            let actual = history(&source, &snapshot, &request, "2026-10-02")?;
            assert_eq!(actual.get("consistent"), Some(&json!(*attempts == Some(3))));
            assert_eq!(actual.get("reservations"), Some(&json!([1, 2, 3])));
            assert_eq!(actual.get("recorded_outcomes"), Some(&json!([1, 2])));
            Ok(())
        })
}

#[test]
fn qualification_rejects_physical_replay_admission_even_when_settlement_is_unchanged() -> Result<()>
{
    let (source, _, _) = exhausted()?;
    let (mut run, _, request) = exhausted()?;
    run.id = "source2".into();
    let mut repeat = SourceRepeat {
        original_id: source.id.clone(),
        input: census_service::restate_services::TeamsSourceRequest {
            jurisdiction: request,
            source: "milesplit".into(),
            observed_on: "2026-10-02".into(),
        },
        run,
        before: vec![json!({"sequence":1})],
        after: vec![json!({"sequence":1})],
    };
    assert_eq!(replay_matches(&source, &repeat)?, true);
    repeat
        .after
        .push(json!({"sequence":2, "response_status":null}));
    assert_eq!(replay_matches(&source, &repeat)?, false);
    Ok(())
}

#[test]
fn qualification_accepts_permanent_settlement_only_with_one_paired_attempt() -> Result<()> {
    let (mut source, mut snapshot, request) = exhausted()?;
    let progress = serde_json::from_value(json!([
        {"status":"terminal", "attempt":1, "outcome":null, "message":"parser refused source"}
    ]))?;
    source.outcome = Some(TeamsSourceOutcome::Terminal {
        message: "parser refused source".into(),
        progress,
    });
    source.shared_state.body = serde_json::to_value(&source.outcome)?;
    snapshot.entries.insert(
        format!("{}/settled", source.key),
        source.shared_state.body.clone(),
    );
    snapshot.entries.insert(
        format!("{}/attempt/1/outcome", source.key),
        json!({"status":"terminal", "message":"parser refused source"}),
    );
    [2, 3].iter().for_each(|attempt| {
        snapshot
            .entries
            .remove(&format!("{}/attempt/{attempt}/reserved", source.key));
        snapshot
            .entries
            .remove(&format!("{}/attempt/{attempt}/outcome", source.key));
    });
    let actual = history(&source, &snapshot, &request, "2026-10-02")?;
    assert_eq!(actual.get("consistent"), Some(&json!(true)));
    assert_eq!(actual.get("reservations"), Some(&json!([1])));
    assert_eq!(actual.get("recorded_outcomes"), Some(&json!([1])));
    snapshot.entries.insert(
        format!("{}/attempt/2/reserved", source.key),
        json!({"attempt":2, "observed_on":"2026-10-02"}),
    );
    let invalid = history(&source, &snapshot, &request, "2026-10-02")?;
    assert_eq!(invalid.get("consistent"), Some(&json!(false)));
    Ok(())
}

#[test]
fn qualification_refuses_exhaustion_when_prior_outcome_is_terminal_or_not_typed_transient(
) -> Result<()> {
    [
        json!({"status":"terminal", "message":"already refused"}),
        json!({"status":"completed", "outcome":{"records":4}}),
        json!({"status":"transient"}),
        json!({"status":"transient", "message":404}),
    ]
    .iter()
    .try_for_each(|value| -> Result<()> {
        let (source, mut snapshot, request) = exhausted()?;
        snapshot
            .entries
            .insert(format!("{}/attempt/1/outcome", source.key), value.clone());
        let certified = history(&source, &snapshot, &request, "2026-10-02")
            .ok()
            .and_then(|actual| actual.get("consistent").cloned());
        assert_ne!(certified, Some(json!(true)));
        Ok(())
    })
}

#[test]
fn qualification_classifies_unknown_then_transient_budget_as_interrupted_not_exhausted(
) -> Result<()> {
    let (mut source, mut snapshot, request) = exhausted()?;
    snapshot
        .entries
        .remove(&format!("{}/attempt/1/outcome", source.key));
    let progress = serde_json::from_value(json!([
        {"status":"unknown", "attempt":1},
        {"status":"transient", "attempt":2, "outcome":null, "message":"actual TLS EOF 2"},
        {"status":"transient", "attempt":3, "outcome":null, "message":"actual TLS EOF 3"}
    ]))?;
    source.outcome = Some(TeamsSourceOutcome::Interrupted {
        attempts: Some(3),
        message: "uncertain earlier acquisition".into(),
        progress,
    });
    source.shared_state.body = serde_json::to_value(&source.outcome)?;
    snapshot.entries.insert(
        format!("{}/settled", source.key),
        source.shared_state.body.clone(),
    );
    let actual = history(&source, &snapshot, &request, "2026-10-02")?;
    assert_eq!(actual.get("consistent"), Some(&json!(true)));
    assert_eq!(actual.get("reservations"), Some(&json!([1, 2, 3])));
    assert_eq!(actual.get("recorded_outcomes"), Some(&json!([2, 3])));
    Ok(())
}

#[test]
fn qualification_refuses_settled_progress_that_does_not_match_actual_attempt_slots() -> Result<()> {
    let (mut source, mut snapshot, request) = exhausted()?;
    source.shared_state.body["progress"][0]["attempt"] = json!(2);
    source.outcome = Some(serde_json::from_value(source.shared_state.body.clone())?);
    snapshot.entries.insert(
        format!("{}/settled", source.key),
        source.shared_state.body.clone(),
    );
    let actual = history(&source, &snapshot, &request, "2026-10-02")?;
    assert_eq!(actual.get("consistent"), Some(&json!(false)));
    assert_eq!(
        actual.get("per_attempt_progress_matches_actual_slots"),
        Some(&json!(false))
    );
    Ok(())
}
