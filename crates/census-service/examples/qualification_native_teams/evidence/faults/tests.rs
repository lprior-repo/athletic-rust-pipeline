use super::*;
use census_domain::model::serialized_digest;
use census_service::restate_services::TeamsSourceRequest;
use std::collections::BTreeMap;

use crate::qualification_native_teams::{http::Observation, ledger, scenario};

fn boundary() -> Result<(Snapshot, String, TeamsSourceRequest, Value)> {
    let (parent, request) = scenario::request()?;
    let input = TeamsSourceRequest {
        jurisdiction: serde_json::from_value(request)?,
        source: "milesplit".into(),
        observed_on: "2026-10-02".into(),
    };
    let key = format!("{parent}/teams/milesplit");
    let digest = serialized_digest(&(
        input.jurisdiction.jurisdiction,
        input.jurisdiction.season,
        input.jurisdiction.revision,
        &input.source,
    ))?;
    let mut entries = BTreeMap::new();
    entries.insert(
        format!("{key}/identity"),
        json!({"request_digest":digest, "observed_on":input.observed_on}),
    );
    (1..=3).for_each(|attempt| {
        entries.insert(
            format!("{key}/attempt/{attempt}/reserved"),
            json!({"attempt":attempt, "observed_on":input.observed_on}),
        );
    });
    (1..=2).for_each(|attempt| {
        entries.insert(
            format!("{key}/attempt/{attempt}/outcome"),
            json!({"status":"transient", "message":format!("EOF {attempt}"), "progress":null}),
        );
    });
    let progress = json!([
        {"status":"transient", "attempt":1, "outcome":null, "message":"EOF 1"},
        {"status":"transient", "attempt":2, "outcome":null, "message":"EOF 2"},
        {"status":"unknown", "attempt":3}
    ]);
    Ok((
        Snapshot {
            phase: ledger::PHASE,
            sequence: 9,
            entries,
        },
        key,
        input,
        progress,
    ))
}

#[test]
fn unknown_third_requires_correlated_ingress_progress_and_exact_cold_slots() -> Result<()> {
    let (mut cold, key, input, mut progress) = boundary()?;
    assert!(slots::boundary(&cold, &key, &input, &progress)?);
    progress[2] = json!({"status":"unknown", "attempt":2});
    assert!(!slots::boundary(&cold, &key, &input, &progress)?);
    let (_, _, _, progress) = boundary()?;
    cold.entries.insert(
        format!("{key}/attempt/3/outcome"),
        json!({"status":"transient", "message":"hold ended before exit", "progress":null}),
    );
    assert!(!slots::boundary(&cold, &key, &input, &progress)?);
    Ok(())
}

#[test]
fn cold_witness_refuses_extra_reservation_missing_pair_and_another_operation() -> Result<()> {
    ["attempt/4/reserved", "settled", "attempt/03/reserved"]
        .iter()
        .try_for_each(|suffix| -> Result<()> {
            let (mut cold, key, input, progress) = boundary()?;
            cold.entries
                .insert(format!("{key}/{suffix}"), json!({"attempt":4}));
            assert!(
                !slots::boundary(&cold, &key, &input, &progress)?,
                "{suffix}"
            );
            Ok(())
        })?;
    let (mut cold, key, input, progress) = boundary()?;
    cold.entries.remove(&format!("{key}/attempt/2/outcome"));
    assert!(!slots::boundary(&cold, &key, &input, &progress)?);
    let (cold, key, mut input, progress) = boundary()?;
    input.jurisdiction.revision = census_reconcile::identity::Revision(99);
    assert!(!slots::boundary(&cold, &key, &input, &progress)?);
    Ok(())
}

#[test]
fn interrupted_recovery_refuses_changed_slots_or_late_third_outcome() -> Result<()> {
    let (before, key, _, _) = boundary()?;
    let (mut after, _, _, _) = boundary()?;
    after.entries.insert(
        format!("{key}/settled"),
        json!({"status":"interrupted", "attempts":3}),
    );
    assert!(slots::unchanged(&before, &after, &key));
    after.entries.insert(
        format!("{key}/attempt/3/outcome"),
        json!({"status":"transient"}),
    );
    assert!(!slots::unchanged(&before, &after, &key));
    after.entries.remove(&format!("{key}/attempt/3/outcome"));
    after.entries.insert(
        format!("{key}/attempt/1/outcome"),
        json!({"status":"terminal"}),
    );
    assert!(!slots::unchanged(&before, &after, &key));
    Ok(())
}

#[test]
fn held_witness_requires_term_after_inspection_and_restart_after_reap() {
    let held = json!({"accepted_at":"10.000000000Z-unix"});
    let inspection = Observation {
        method: "POST".into(),
        url: "http://127.0.0.1/".into(),
        sent_at: "11.000000000Z-unix".into(),
        received_at: "12.000000000Z-unix".into(),
        status: 200,
        error_source: None,
        invocation_id: None,
        body: json!([]),
    };
    let mut cleanup = json!({"endpoint_term_reap":{"term_at":"13.000000000Z-unix",
        "success":true, "reaped":true}, "drain_certificate":{"balanced":true},
        "reaped_at":"14.000000000Z-unix", "sigkill_used":false});
    let release = json!({"at":"15.000000000Z-unix"});
    let mut restart = json!({"at":"16.000000000Z-unix", "configuration_changed":false,
        "native_node_restarted":false, "old_owner_reaped":cleanup});
    assert!(timeline::ordered(
        &held,
        &inspection,
        &cleanup,
        &release,
        &restart
    ));
    restart["at"] = json!("13.000000000Z-unix");
    assert!(!timeline::ordered(
        &held,
        &inspection,
        &cleanup,
        &release,
        &restart
    ));
    restart["at"] = json!("16.000000000Z-unix");
    cleanup["endpoint_term_reap"]["term_at"] = json!("11.000000000Z-unix");
    restart["old_owner_reaped"] = cleanup.clone();
    assert!(!timeline::ordered(
        &held,
        &inspection,
        &cleanup,
        &release,
        &restart
    ));
}

#[test]
fn native_progress_witness_cannot_target_a_flattened_or_other_source_key() -> Result<()> {
    let key = "OH/2026/1/teams/milesplit";
    let mut url = url::Url::parse("http://127.0.0.1:8080/")?;
    url.path_segments_mut()
        .map_err(|()| anyhow::anyhow!("no URL segments"))?
        .extend(["TeamsSource", key, "progress"]);
    assert!(progress_route(url.as_str(), key));
    assert!(!progress_route(url.as_str(), "OH/2026/2/teams/milesplit"));
    assert!(!progress_route(
        "http://127.0.0.1:8080/restate/call/TeamsSource/OH/2026/1/teams/milesplit/progress",
        key
    ));
    Ok(())
}

#[test]
fn missing_original_fault_obligations_keep_full_qualification_unproven() -> Result<()> {
    let checks = checks(&Faults::default(), None)?;
    assert!(checks
        .iter()
        .all(|check| check.get("status") == Some(&json!("UNPROVEN"))));
    assert!(
        checks
            .iter()
            .any(|check| check.get("name")
                == Some(&json!("permanent_fetch_network_response_refusal")))
    );
    assert!(checks.iter().any(|check| check.get("name")
        == Some(&json!(
            "full_machine_reset_and_real_clock_fault_qualification"
        ))));
    assert!(checks.iter().any(|check| check.get("name")
        == Some(&json!("fresh_positive_source_acquisition_qualification"))));
    Ok(())
}

#[test]
fn native_direct_identity_requires_positive_sql_null_witnesses_when_fields_are_omitted() {
    let key = "jurisdiction:OH:2026-27:2/teams/milesplit";
    let id = "inv_direct";
    let mut row = json!({"id":id, "status":"completed", "completion_result":"success",
        "target_handler_name":"run", "target_service_key":key, "target_service_name":"TeamsSource"});
    assert!(!direct(&row, key, id));
    row["invoked_by"] = json!("ingress");
    assert!(!direct(&row, key, id));
    row["caller_service_absent"] = json!(true);
    assert!(!direct(&row, key, id));
    row["caller_id_absent"] = json!(true);
    assert!(direct(&row, key, id));
    row["invoked_by_service_name"] = Value::Null;
    row["invoked_by_id"] = Value::Null;
    assert!(direct(&row, key, id));
    assert!(!direct(
        &row,
        "jurisdiction:OH:2026-27:3/teams/milesplit",
        id
    ));
    assert!(!direct(&row, key, "inv_other"));
}

#[test]
fn native_direct_identity_refuses_service_callers_and_inconsistent_null_projections() {
    let key = "jurisdiction:OH:2026-27:3/teams/milesplit";
    let id = "inv_direct";
    let row = json!({"id":id, "status":"running", "target_handler_name":"run",
        "target_service_key":key, "target_service_name":"TeamsSource", "invoked_by":"ingress",
        "caller_service_absent":true, "caller_id_absent":true});
    assert!(direct(&row, key, id));
    [
        ("invoked_by", json!("service")),
        ("invoked_by", json!("subscription")),
        ("invoked_by", json!("restart_as_new")),
        ("caller_service_absent", json!(false)),
        ("caller_id_absent", json!(false)),
        ("caller_id_absent", Value::Null),
        ("invoked_by_service_name", json!("JurisdictionCensus")),
        ("invoked_by_id", json!("inv_parent")),
        ("target_handler_name", json!("progress")),
        ("target_service_name", json!("JurisdictionCensus")),
    ]
    .into_iter()
    .for_each(|(field, value)| {
        let mut changed = row.clone();
        changed[field] = value;
        assert!(!direct(&changed, key, id), "{field}");
    });
}
