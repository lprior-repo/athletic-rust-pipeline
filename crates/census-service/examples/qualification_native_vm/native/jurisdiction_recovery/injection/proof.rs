use super::super::input::Original;
use super::super::journal::SourceCall;
use super::super::observe::Observation;
use super::super::{input, journal as source_journal, observe};
use super::{configuration, files, journal, Configuration, Marker, MARKER};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn witness_at(
    path: &Path,
    observation: &Observation,
    original: &Original,
    mut witness: Value,
) -> Result<Option<Value>> {
    let expected = configuration(&original.request, &original.source_plan)?;
    ensure!(
        original.key == input::identity(&original.request),
        "native marker original identity differs"
    );
    let config_bytes = files::read_required(path)?;
    let config: Configuration =
        serde_json::from_slice(&config_bytes).context("native boundary config malformed")?;
    ensure!(
        config == expected,
        "native boundary config differs from original request or fixed attempt/timeout"
    );
    let call: SourceCall = serde_json::from_value(
        witness
            .get("source_call")
            .context("runtime source call absent")?
            .clone(),
    )?;
    ensure!(
        call.child_key == config.operation,
        "native marker cannot certify an unconfigured source child"
    );
    let attempt = u8::try_from(
        witness
            .get("source_attempt")
            .and_then(Value::as_u64)
            .context("runtime reservation attempt absent")?,
    )?;
    let marker_path = path.with_file_name(MARKER);
    let Some(marker_bytes) = files::published_marker(&marker_path)? else {
        return Ok(None);
    };
    let marker: Marker = serde_json::from_slice(&marker_bytes)
        .context("published native reservation marker malformed")?;
    let Some((registration, references)) = registration(observation, &call)? else {
        return Ok(None);
    };
    agrees(&config, &marker, &call, attempt, &registration)?;
    witness.as_object_mut().context("source witness malformed")?.insert(
        "native_source_boundary".to_owned(),
        json!({"configuration":files::evidence(path, &config_bytes)?,"marker":files::evidence(&marker_path, &marker_bytes)?,"registration":registration,"registration_journal_references":references,"proof":"reserved_before_acquisition","physical_request_in_flight_proven":false}),
    );
    Ok(Some(witness))
}

fn registration(
    observation: &Observation,
    call: &SourceCall,
) -> Result<Option<(journal::Registration, Value)>> {
    let child = observation
        .children
        .iter()
        .find(|child| child.id == call.child_id)
        .context("native marker original child absent")?;
    ensure!(
        child.key == call.child_key,
        "native marker original child key differs"
    );
    let status = observe::status(&observation.before, &call.child_id)?;
    let Some(entries) = source_journal::complete(&child.journal, status, &call.child_id)? else {
        return Ok(None);
    };
    source_journal::child_input(entries, call)?;
    journal::registration(entries)
}

fn agrees(
    config: &Configuration,
    marker: &Marker,
    call: &SourceCall,
    attempt: u8,
    registration: &journal::Registration,
) -> Result<()> {
    ensure!(
        marker.schema == 1 && marker.phase == "teams_reserved_before_acquisition",
        "native marker schema/phase differs"
    );
    ensure!(
        marker.operation == config.operation && marker.operation == call.child_key,
        "native marker operation differs from original source child"
    );
    ensure!(
        marker.attempt == config.attempt && marker.attempt == attempt,
        "native marker attempt differs from actual reservation"
    );
    ensure!(
        marker.request_digest == registration.request_digest,
        "native marker digest differs from journaled registration"
    );
    ensure!(
        marker.observed_on == registration.observed_on
            && marker.observed_on == call.request.observed_on,
        "native marker observation date differs from original journaled registration/request"
    );
    Ok(())
}

pub(super) fn retained_at(path: &Path, original: &Original, before: &Value) -> Result<Value> {
    let observation: Observation = serde_json::from_value(
        before
            .pointer("/boundary/observation")
            .context("pre-reset source observation absent")?
            .clone(),
    )?;
    let witness = before
        .pointer("/boundary/witness")
        .context("pre-reset source witness absent")?;
    let retained = witness_at(path, &observation, original, witness.clone())?.context(
        "retained native reservation marker missing or incompletely published after reboot",
    )?;
    let expected = witness
        .get("native_source_boundary")
        .context("pre-reset native marker evidence absent")?;
    let current = retained
        .get("native_source_boundary")
        .context("retained native marker evidence absent")?;
    ensure!(
        expected == current,
        "native config/marker or registration evidence changed across reboot"
    );
    Ok(
        json!({"original_invocation_id":original.id,"retained":current,"same_private_artifacts":true,"replacement_submitted":false,"attempt_reset":false}),
    )
}
