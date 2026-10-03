use super::super::super::input::Original;
use super::super::super::observe::{Child, Observation};
use anyhow::{Context, Result};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_service::restate_services::{
    JurisdictionRequest, JurisdictionState, SourcePlan, TeamsAttemptProgress,
    TeamsSourceInspection, TeamsSourceRequest, TeamsStage,
};
use serde_json::{json, Value};

pub(super) fn active() -> Result<(Original, Observation)> {
    let request = JurisdictionRequest {
        jurisdiction: UsJurisdiction::RhodeIsland,
        season: SchoolYear::new(2026).context("invalid test season")?,
        revision: Revision(1),
        refresh: false,
        limit_per_state: Some(1),
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    };
    let key =
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision)
            .to_string();
    let plan = SourcePlan {
        sweepable: vec!["milesplit".to_owned(), "riil".to_owned()],
        refused: Vec::new(),
        fingerprint: "retained-plan".to_owned(),
    };
    let clock = json!({"realtime":"2026-10-02T00:00:00Z","boot_id":"original-boot","machine_id":"owned-machine"});
    let original = Original {
        id: "inv_parent".to_owned(),
        key: key.clone(),
        request: request.clone(),
        accepted: json!({"invocationId":"inv_parent"}),
        clock: clock.clone(),
        manifest: json!({}),
        deployment: json!({"id":"dp_owned"}),
        owner: json!({}),
        source_plan: plan.clone(),
        captures_before: Vec::new(),
    };
    let source = TeamsSourceRequest {
        jurisdiction: request,
        source: "milesplit".to_owned(),
        observed_on: "2026-10-02".to_owned(),
    };
    let child_key = format!("{key}/teams/milesplit");
    let parent_journal = json!([
        entry(
            "inv_parent",
            0,
            "Command: Input",
            json!({"Command":{"Input":{"payload":serde_json::to_vec(&original.request)?}}})
        ),
        entry(
            "inv_parent",
            1,
            "Command: Call",
            json!({"Command":{"Call":{
                "request":{"invocation_id":"inv_child","invocation_target":{"VirtualObject":{"name":"TeamsSource","key":child_key,"handler":"run","handler_ty":"Exclusive"}},"parameter":serde_json::to_vec(&source)?},
                "invocation_id_completion_id":1,"result_completion_id":2
            }}})
        ),
        entry(
            "inv_parent",
            2,
            "Notification: CallInvocationId",
            json!({"Notification":{"Completion":{"CallInvocationId":{"completion_id":1,"invocation_id":"inv_child"}}}})
        )
    ]);
    let before = json!([
        status("inv_child", &child_key, "TeamsSource", "running", 1),
        status("inv_parent", &key, "JurisdictionCensus", "suspended", 3)
    ]);
    let child = Child {
        id: "inv_child".to_owned(),
        key: child_key,
        inspection_before: inspection(),
        inspection_after: inspection(),
        journal: json!([entry(
            "inv_child",
            0,
            "Command: Input",
            json!({"Command":{"Input":{"payload":serde_json::to_vec(&source)?}}})
        )]),
    };
    let observation = Observation {
        before: before.clone(),
        after: before,
        parent_journal,
        children: vec![child],
        clock,
        parent_state: JurisdictionState {
            identity: key,
            plan: Some(plan),
            teams: TeamsStage::Owed,
            ..JurisdictionState::default()
        },
    };
    Ok((original, observation))
}

fn inspection() -> TeamsSourceInspection {
    TeamsSourceInspection::Unsettled {
        progress: vec![TeamsAttemptProgress::Unknown { attempt: 1 }],
    }
}

fn status(id: &str, key: &str, service: &str, status: &str, journal_size: u64) -> Value {
    json!({"id":id,"target_service_name":service,"target_service_key":key,"target_handler_name":"run","status":status,"pinned_deployment_id":"dp_owned","pinned_service_protocol_version":7,"restarted_from":null,"journal_size":journal_size,"invoked_by":"service","invoked_by_id":"inv_parent","suspended_waiting_future_json":{"Single":{"CompletionId":2}}})
}

pub(super) fn entry(id: &str, index: u64, kind: &str, entry: Value) -> Value {
    json!({"id":id,"index":index,"version":2,"entry_type":kind,"entry_json":entry})
}

pub(super) fn journal_size(observation: &mut Value, id: &str, size: u64) -> Result<()> {
    status_field(observation, id, "journal_size", json!(size))
}

pub(super) fn status_field(
    observation: &mut Value,
    id: &str,
    field: &str,
    value: Value,
) -> Result<()> {
    ["before", "after"]
        .into_iter()
        .try_for_each(|bookend| -> Result<()> {
            let rows = observation
                .get_mut(bookend)
                .and_then(Value::as_array_mut)
                .context("status bookend absent")?;
            let row = rows
                .iter_mut()
                .find(|row| row.get("id").and_then(Value::as_str) == Some(id))
                .context("status row absent")?;
            row.as_object_mut()
                .context("status row malformed")?
                .insert(field.to_owned(), value.clone());
            Ok(())
        })
}
