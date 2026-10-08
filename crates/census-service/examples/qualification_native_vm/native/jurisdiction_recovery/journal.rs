use super::super::http::rows;
use super::input::{self, Original};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::TeamsSourceRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::BTreeSet;

mod admission;
pub(super) use admission::settled_sources;

mod future;
mod ledger;
pub(super) use future::awaited;
pub(super) use ledger::source_ledger;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JournalRef {
    pub(super) invocation_id: String,
    pub(super) index: u64,
    pub(super) version: u64,
}

#[derive(Serialize, Deserialize)]
pub(super) struct SourceCall {
    pub(super) command: JournalRef,
    pub(super) child_id: String,
    pub(super) child_key: String,
    pub(super) invocation_completion: u32,
    pub(super) result_completion: u32,
    pub(super) request: TeamsSourceRequest,
}

#[derive(Serialize)]
pub(super) struct StoreJournalRef {
    phase: &'static str,
    key: String,
}

pub(super) fn complete<'a>(
    journal: &'a Value,
    status: &Value,
    id: &str,
) -> Result<Option<&'a [Value]>> {
    let journal = rows(journal)?;
    ensure!(
        journal.len() <= 2048,
        "source journal exceeds 2048-entry evidence budget"
    );
    let size = status
        .get("journal_size")
        .and_then(Value::as_u64)
        .context("journal size absent")?;
    if u64::try_from(journal.len())? != size {
        return Ok(None);
    }
    journal
        .iter()
        .enumerate()
        .try_for_each(|(index, row)| -> Result<()> {
            ensure!(input::text(row, "id")? == id, "foreign journal invocation");
            ensure!(
                row.get("index").and_then(Value::as_u64) == Some(u64::try_from(index)?),
                "incomplete journal index sequence"
            );
            ensure!(
                row.get("version").and_then(Value::as_u64) == Some(2),
                "source witness requires journal v2"
            );
            let entry = decode(row.get("entry_json").context("journal JSON absent")?)?;
            let (kind, payload) = variant(&entry)?;
            ensure!(
                matches!(kind, "Command" | "Notification"),
                "unknown journal envelope"
            );
            variant(payload)?;
            Ok(())
        })?;
    Ok(Some(journal))
}

pub(super) fn calls(journal: &[Value], original: &Original) -> Result<Vec<SourceCall>> {
    journal.iter().try_fold(Vec::new(), |mut calls, row| {
        let entry = decode(row.get("entry_json").context("journal JSON absent")?)?;
        if let Some(call) = source_call(row, &entry, original)? {
            ensure!(calls.len() < 16, "source call budget exceeded");
            ensure!(
                calls.iter().all(|prior: &SourceCall| prior.child_id != call.child_id
                    && prior.child_key != call.child_key),
                "duplicate original source call identity"
            );
            calls.try_reserve(1)?;
            calls.push(call);
        }
        Ok(calls)
    })
}

fn source_call(row: &Value, entry: &Value, original: &Original) -> Result<Option<SourceCall>> {
    let Some(call) = entry.pointer("/Command/Call") else {
        return Ok(None);
    };
    ensure!(
        input::text(row, "entry_type")? == "Command: Call",
        "source call journal type mismatch"
    );
    let request = call.get("request").context("call request absent")?;
    let Some(target) = request.pointer("/invocation_target/VirtualObject") else {
        return Ok(None);
    };
    if input::text(target, "name")? != "TeamsSource" || input::text(target, "handler")? != "run" {
        return Ok(None);
    }
    ensure!(
        input::text(target, "handler_ty")? == "Exclusive",
        "source call is not exclusive"
    );
    let input: TeamsSourceRequest = serde_json::from_slice(&bytes(
        request
            .get("parameter")
            .context("source request parameter absent")?,
    )?)?;
    ensure!(
        serde_json::to_value(&input.jurisdiction)? == serde_json::to_value(&original.request)?,
        "source request semantics differ from accepted parent"
    );
    ensure!(
        original.source_plan.sweepable.contains(&input.source),
        "unplanned source child"
    );
    let key = input::text(target, "key")?;
    ensure!(
        key == format!("{}/teams/{}", original.key, input.source),
        "source unit identity mismatch"
    );
    let child_id = input::text(request, "invocation_id")?.to_owned();
    input::safe_id(&child_id)?;
    Ok(Some(SourceCall {
        command: reference(row)?,
        child_id,
        child_key: key.to_owned(),
        request: input,
        invocation_completion: completion(call, "invocation_id_completion_id")?,
        result_completion: completion(call, "result_completion_id")?,
    }))
}

pub(super) fn parent_input(journal: &[Value], original: &Original) -> Result<()> {
    let first = journal.first().context("parent Input journal absent")?;
    let entry = decode(
        first
            .get("entry_json")
            .context("parent Input JSON absent")?,
    )?;
    let payload = entry
        .pointer("/Command/Input/payload")
        .context("parent journal does not begin with Input")?;
    let request: census_service::restate_services::JurisdictionRequest =
        serde_json::from_slice(&bytes(payload)?)?;
    ensure!(
        serde_json::to_value(request)? == serde_json::to_value(&original.request)?,
        "original journal input differs from accepted request"
    );
    Ok(())
}

pub(super) fn child_input(journal: &[Value], call: &SourceCall) -> Result<()> {
    let first = journal.first().context("child Input journal absent")?;
    let entry = decode(first.get("entry_json").context("child Input JSON absent")?)?;
    let payload = entry
        .pointer("/Command/Input/payload")
        .context("child journal does not begin with Input")?;
    let request: TeamsSourceRequest = serde_json::from_slice(&bytes(payload)?)?;
    ensure!(
        serde_json::to_value(request)? == serde_json::to_value(&call.request)?,
        "original child input differs from parent source call"
    );
    Ok(())
}

pub(super) fn completions(journal: &[Value]) -> Result<BTreeSet<u32>> {
    journal.iter().try_fold(BTreeSet::new(), |mut ids, row| {
        let entry = decode(row.get("entry_json").context("journal JSON absent")?)?;
        if let Some(value) = entry.pointer("/Notification/Completion") {
            let (kind, payload) = variant(value)?;
            ensure!(
                input::text(row, "entry_type")?.strip_prefix("Notification: ") == Some(kind),
                "journal completion type mismatch"
            );
            ensure!(
                ids.insert(completion(payload, "completion_id")?),
                "duplicate journal completion ID"
            );
        }
        Ok(ids)
    })
}

pub(super) fn invocation_ack(journal: &[Value], call: &SourceCall) -> Result<bool> {
    journal.iter().try_fold(false, |found, row| {
        let entry = decode(row.get("entry_json").context("journal JSON absent")?)?;
        let Some(value) = entry.pointer("/Notification/Completion/CallInvocationId") else {
            return Ok(found);
        };
        if completion(value, "completion_id")? != call.invocation_completion {
            return Ok(found);
        }
        ensure!(
            input::text(value, "invocation_id")? == call.child_id,
            "journal acknowledged another source child"
        );
        Ok(true)
    })
}

pub(super) fn reference(row: &Value) -> Result<JournalRef> {
    Ok(JournalRef {
        invocation_id: input::text(row, "id")?.to_owned(),
        index: row
            .get("index")
            .and_then(Value::as_u64)
            .context("journal index absent")?,
        version: row
            .get("version")
            .and_then(Value::as_u64)
            .context("journal version absent")?,
    })
}

pub(super) fn decode(value: &Value) -> Result<Cow<'_, Value>> {
    match value {
        Value::String(raw) => Ok(Cow::Owned(serde_json::from_str(raw)?)),
        Value::Object(_) => Ok(Cow::Borrowed(value)),
        _ => anyhow::bail!("journal JSON is neither object nor encoded JSON"),
    }
}

fn variant(value: &Value) -> Result<(&str, &Value)> {
    let object = value.as_object().context("journal variant malformed")?;
    ensure!(object.len() == 1, "ambiguous journal variant");
    object
        .iter()
        .next()
        .map(|(kind, value)| (kind.as_str(), value))
        .context("journal variant absent")
}

fn completion(value: &Value, field: &str) -> Result<u32> {
    Ok(u32::try_from(
        value
            .get(field)
            .and_then(Value::as_u64)
            .with_context(|| format!("{field} absent"))?,
    )?)
}

fn bytes(value: &Value) -> Result<Vec<u8>> {
    let values = value.as_array().context("journal byte payload malformed")?;
    ensure!(
        values.len() <= 64 * 1024,
        "source request payload exceeds 64 KiB"
    );
    values
        .iter()
        .map(|value| {
            Ok(u8::try_from(
                value.as_u64().context("journal byte malformed")?,
            )?)
        })
        .collect()
}
