use super::super::super::input::Original;
use super::super::super::observe::{Child, Observation};
use anyhow::{Context, Result};
use census_service::restate_services::{
    StageOutcome, TeamsAttemptProgress, TeamsSourceInspection, TeamsSourceOutcome,
    TeamsSourceRequest,
};
use serde_json::{json, Value};

pub(in super::super::super) fn completed() -> Result<(Original, Observation)> {
    let (original, mut observation) = super::super::super::injection::tests::fixture::active()?;
    let first = observation
        .parent_journal
        .as_array()
        .context("parent journal absent")?
        .first()
        .context("parent Input absent")?
        .clone();
    let child_journal = observation
        .children
        .first()
        .context("original child absent")?
        .journal
        .clone();
    let mut parent_journal = vec![first];
    let mut statuses = Vec::new();
    let mut children = Vec::new();
    for (id, source, invocation_completion, result_completion) in [
        ("inv_child", "milesplit", 7, 8),
        ("inv_sibling", "riil", 9, 10),
    ] {
        let request = TeamsSourceRequest {
            jurisdiction: original.request.clone(),
            source: source.to_owned(),
            observed_on: "2026-10-02".to_owned(),
        };
        let key = format!("{}/teams/{source}", original.key);
        let outcome = outcome(source);
        append_source(
            &mut parent_journal,
            id,
            &key,
            &request,
            (invocation_completion, result_completion),
            &outcome,
        )?;
        statuses.push(status(id, &key, "TeamsSource", 3));
        children.push(child(id, key, request, outcome, &child_journal)?);
    }
    statuses.push(status(
        &original.id,
        &original.key,
        "JurisdictionCensus",
        u64::try_from(parent_journal.len())?,
    ));
    observation.before = json!(statuses);
    observation.after = observation.before.clone();
    observation.parent_journal = json!(parent_journal);
    observation.children = children;
    Ok((original, observation))
}

fn outcome(source: &str) -> TeamsSourceOutcome {
    let outcome = StageOutcome {
        records: if source == "milesplit" { 3 } else { 5 },
        at: "2026-10-02".to_owned(),
        errors: Vec::new(),
        notes: Vec::new(),
        disposition: Default::default(),
        unfinished: Vec::new(),
    };
    TeamsSourceOutcome::Completed {
        progress: vec![TeamsAttemptProgress::Completed {
            attempt: 1,
            outcome: outcome.clone(),
        }],
        outcome,
    }
}

fn append_source(
    entries: &mut Vec<Value>,
    id: &str,
    key: &str,
    request: &TeamsSourceRequest,
    completions: (u32, u32),
    outcome: &TeamsSourceOutcome,
) -> Result<()> {
    let (invocation_completion, result_completion) = completions;
    push(
        entries,
        "Command: Call",
        json!({"Command":{"Call":{
            "request":{"invocation_id":id,"invocation_target":{"VirtualObject":{
                "name":"TeamsSource","key":key,"handler":"run","handler_ty":"Exclusive"
            }},"parameter":serde_json::to_vec(request)?},
            "invocation_id_completion_id":invocation_completion,"result_completion_id":result_completion
        }}}),
    )?;
    push(
        entries,
        "Notification: CallInvocationId",
        json!({"Notification":{"Completion":{
            "CallInvocationId":{"completion_id":invocation_completion,"invocation_id":id}
        }}}),
    )?;
    push(
        entries,
        "Notification: Call",
        json!({"Notification":{"Completion":{
            "Call":{"completion_id":result_completion,"result":{"Success":serde_json::to_vec(outcome)?}}
        }}}),
    )
}

fn push(entries: &mut Vec<Value>, kind: &str, entry: Value) -> Result<()> {
    entries.push(row(
        "inv_parent",
        u64::try_from(entries.len())?,
        kind,
        entry,
    ));
    Ok(())
}

fn child(
    id: &str,
    key: String,
    request: TeamsSourceRequest,
    outcome: TeamsSourceOutcome,
    template: &Value,
) -> Result<Child> {
    let inspection = TeamsSourceInspection::Settled { outcome };
    let mut journal = template.clone();
    for row in journal.as_array_mut().context("child journal absent")? {
        *row.get_mut("id").context("journal identity absent")? = json!(id);
    }
    *journal
        .pointer_mut("/0/entry_json/Command/Input/payload")
        .context("child Input absent")? = json!(serde_json::to_vec(&request)?);
    Ok(Child {
        id: id.to_owned(),
        key,
        inspection_before: inspection.clone(),
        inspection_after: inspection,
        journal,
    })
}

fn status(id: &str, key: &str, service: &str, size: u64) -> Value {
    json!({
        "id":id,"target_service_key":key,"target_service_name":service,
        "target_handler_name":"run","status":"completed","completion_result":"success",
        "invoked_by_id":"inv_parent","pinned_deployment_id":"dp_owned",
        "pinned_service_protocol_version":7,"journal_size":size,"restarted_from":null
    })
}

pub(in super::super::super) fn status_field(
    observation: &mut Observation,
    id: &str,
    field: &str,
    value: Value,
) -> Result<()> {
    for statuses in [&mut observation.before, &mut observation.after] {
        let row = statuses
            .as_array_mut()
            .context("statuses absent")?
            .iter_mut()
            .find(|row| row.get("id").and_then(Value::as_str) == Some(id))
            .context("status absent")?;
        row.as_object_mut()
            .context("status malformed")?
            .insert(field.to_owned(), value.clone());
    }
    Ok(())
}

pub(in super::super::super) fn parent_entry(
    observation: &mut Observation,
    index: usize,
) -> Result<&mut Value> {
    observation
        .parent_journal
        .as_array_mut()
        .context("parent journal absent")?
        .get_mut(index)
        .context("parent entry absent")
}

pub(in super::super::super) fn tree(observation: &Observation) -> Result<Vec<Value>> {
    Ok(observation
        .after
        .as_array()
        .context("statuses absent")?
        .clone())
}

pub(in super::super::super) fn active_before(original: &Original) -> Result<Observation> {
    let (_, observation) = super::super::super::injection::tests::fixture::active()?;
    anyhow::ensure!(
        observation.parent_state.identity == original.key,
        "fixture scope differs"
    );
    Ok(observation)
}
