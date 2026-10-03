use super::super::super::{
    boundary,
    input::Original,
    observe::{Child, Observation},
};
use super::super::{files, Marker, MARKER};
use anyhow::{Context, Result};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_reconcile::identity::Revision;
use census_service::restate_services::{
    JurisdictionRequest, JurisdictionState, SourcePlan, TeamsAttemptProgress,
    TeamsSourceInspection, TeamsSourceRequest, TeamsStage,
};
use serde_json::{json, Value};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub(super) fn original(jurisdiction: UsJurisdiction, revision: u32) -> Result<Original> {
    let request = JurisdictionRequest {
        jurisdiction,
        season: SchoolYear::new(2026).context("fixture season invalid")?,
        revision: Revision(revision),
        refresh: false,
        limit_per_state: Some(1),
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    };
    let key = super::super::super::input::identity(&request);
    Ok(Original {
        id: "inv_parent".to_owned(),
        key,
        request,
        accepted: json!({"invocationId":"inv_parent"}),
        clock: json!({"boot_id":"original"}),
        manifest: json!({}),
        deployment: json!({"id":"dp_owned"}),
        owner: json!({}),
        source_plan: SourcePlan {
            sweepable: vec!["milesplit".to_owned(), "riil".to_owned()],
            refused: Vec::new(),
            fingerprint: "owned-plan".to_owned(),
        },
        captures_before: Vec::new(),
    })
}

pub(super) fn active() -> Result<(Original, Observation)> {
    let original = original(UsJurisdiction::RhodeIsland, 17)?;
    let source = TeamsSourceRequest {
        jurisdiction: original.request.clone(),
        source: "milesplit".to_owned(),
        observed_on: "2026-10-02".to_owned(),
    };
    let operation = super::super::operation(&original)?;
    let call = json!({"Command":{"Call":{
        "request":{"invocation_id":"inv_child","invocation_target":{"VirtualObject":{"name":"TeamsSource","key":operation,"handler":"run","handler_ty":"Exclusive"}},"parameter":serde_json::to_vec(&source)?},
        "invocation_id_completion_id":7,"result_completion_id":8
    }}});
    let parent_journal = json!([
        entry(
            "inv_parent",
            0,
            "Command: Input",
            json!({"Command":{"Input":{"payload":serde_json::to_vec(&original.request)?}}})
        ),
        entry("inv_parent", 1, "Command: Call", call),
        entry(
            "inv_parent",
            2,
            "Notification: CallInvocationId",
            json!({"Notification":{"Completion":{"CallInvocationId":{"completion_id":7,"invocation_id":"inv_child"}}}})
        )
    ]);
    let before = json!([
        status(
            "inv_parent",
            &original.key,
            "JurisdictionCensus",
            "suspended"
        ),
        status("inv_child", &operation, "TeamsSource", "running")
    ]);
    let inspection = TeamsSourceInspection::Unsettled {
        progress: vec![TeamsAttemptProgress::Unknown { attempt: 1 }],
    };
    let child = Child {
        id: "inv_child".to_owned(),
        key: operation,
        inspection_before: inspection.clone(),
        inspection_after: inspection,
        journal: json!([
            entry(
                "inv_child",
                0,
                "Command: Input",
                json!({"Command":{"Input":{"payload":serde_json::to_vec(&source)?}}})
            ),
            entry(
                "inv_child",
                1,
                "Command: Run",
                json!({"Command":{"Run":{"completion_id":1,"name":""}}})
            ),
            registration_entry(2, "2026-10-02", &"a".repeat(64))?
        ]),
    };
    let observation = Observation {
        before: before.clone(),
        after: before,
        parent_journal,
        children: vec![child],
        clock: original.clock.clone(),
        parent_state: JurisdictionState {
            identity: original.key.clone(),
            plan: Some(original.source_plan.clone()),
            teams: TeamsStage::Owed,
            ..JurisdictionState::default()
        },
    };
    Ok((original, observation))
}

pub(super) fn status(id: &str, key: &str, service: &str, status: &str) -> Value {
    json!({"id":id,"target_service_key":key,"target_service_name":service,"target_handler_name":"run","status":status,"invoked_by":"service","invoked_by_id":"inv_parent","pinned_deployment_id":"dp_owned","pinned_service_protocol_version":7,"journal_size":3,"restarted_from":null,"suspended_waiting_future_json":{"Single":{"CompletionId":8}}})
}

pub(super) fn entry(id: &str, index: u64, kind: &str, entry: Value) -> Value {
    json!({"id":id,"index":index,"version":2,"entry_type":kind,"entry_json":entry})
}

pub(super) fn registration_entry(index: u64, observed_on: &str, digest: &str) -> Result<Value> {
    let identity = json!({"request_digest":digest,"observed_on":observed_on});
    Ok(entry(
        "inv_child",
        index,
        "Notification: Run",
        json!({"Notification":{"Completion":{"Run":{"completion_id":1,"result":{"Success":serde_json::to_vec(&identity)?}}}}}),
    ))
}

pub(super) fn marker(original: &Original) -> Result<Marker> {
    Ok(Marker {
        schema: 1,
        phase: "teams_reserved_before_acquisition".to_owned(),
        operation: super::super::operation(original)?,
        attempt: 1,
        request_digest: "a".repeat(64),
        observed_on: "2026-10-02".to_owned(),
    })
}

pub(super) fn directory() -> Result<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
    Ok(directory)
}

pub(super) fn configured(root: &Path, original: &Original) -> Result<PathBuf> {
    let path = root.join("native-source-boundary-config.json");
    super::super::prepare_at(
        &path,
        &super::super::configuration(&original.request, &original.source_plan)?,
    )?;
    Ok(path)
}

pub(super) fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn publish_marker(config: &Path, marker: &Marker) -> Result<()> {
    write_private(&config.with_file_name(MARKER), &serde_json::to_vec(marker)?)
}

pub(super) fn runtime(
    config: &Path,
    original: &Original,
    observation: &Observation,
) -> Result<Option<Value>> {
    boundary::assess(observation, original)?
        .map(|witness| super::super::proof::witness_at(config, observation, original, witness))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn completed_parent(observation: &mut Observation) -> Result<()> {
    let update = |value: &mut Value| -> Result<()> {
        let parent = value
            .as_array_mut()
            .context("fixture statuses absent")?
            .iter_mut()
            .find(|row| row.get("id").and_then(Value::as_str) == Some("inv_parent"))
            .context("fixture parent absent")?;
        parent["status"] = json!("completed");
        Ok(())
    };
    update(&mut observation.before)?;
    update(&mut observation.after)
}

pub(super) fn read_config(config: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&files::read_required(config)?)?)
}
