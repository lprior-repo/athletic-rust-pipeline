use super::super::super::http::{request, INGRESS};
use super::super::journal::{self, SourceCall};
use super::super::observe::{self, Child, Observation};
use super::{input, witness, Original};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::TeamsSourceInspection;
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::time::Duration;

pub(super) async fn read_held(client: &Client, original: &Original) -> Result<Observation> {
    let probes = stream::iter(0..120u32)
        .then(|_| async move {
            let current = observe::read(client, original).await?;
            ensure!(
                input::text(observe::status(&current.after, &original.id)?, "status")? == "paused",
                "original parent escaped its owned-admission hold"
            );
            if current.before == current.after {
                return Ok(Some(current));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<_, anyhow::Error>(None)
        })
        .try_filter_map(|value| futures::future::ready(Ok(value)));
    futures::pin_mut!(probes);
    probes
        .try_next()
        .await?
        .context("held original tree did not yield a stable native observation")
}

pub(super) async fn wait(
    client: &Client,
    original: &Original,
    call: &SourceCall,
    previous: &Observation,
) -> Result<Observation> {
    let checks = stream::iter(0..3_200u32)
        .then(|attempt| async move {
            let status = observe::statuses(client, &original.id).await?;
            ensure!(
                input::text(observe::status(&status, &original.id)?, "status")? == "paused",
                "original parent escaped its owned-admission hold"
            );
            let source = observe::status(&status, &call.child_id)?;
            observe::publish(
                "owned-source-wait-latest",
                &json!({"attempt":attempt,"statuses":status}),
            )?;
            if input::text(source, "status")? == "completed" {
                ensure!(
                    input::text(source, "completion_result")? == "success",
                    "original source failed at native invocation layer"
                );
                let current = read_held(client, original).await?;
                witness::held(original, call, previous, &current)?;
                settled(call, &current)?;
                return Ok(Some(current));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok::<_, anyhow::Error>(None)
        })
        .try_filter_map(|value| futures::future::ready(Ok(value)));
    futures::pin_mut!(checks);
    checks
        .try_next()
        .await?
        .context("original source did not settle within existing recovery poll bound")
}

pub(super) async fn output(
    client: &Client,
    call: &SourceCall,
    observation: &Observation,
) -> Result<Value> {
    let child = settled(call, observation)?;
    let TeamsSourceInspection::Settled { outcome } = &child.inspection_after else {
        anyhow::bail!("original source has no settled output");
    };
    let output = request(
        client,
        Method::GET,
        &format!("{INGRESS}restate/invocation/{}/output", call.child_id),
        None,
    )
    .await?;
    ensure!(
        output == serde_json::to_value(outcome)?,
        "original native source output differs from retained production inspection"
    );
    Ok(output)
}

pub(super) fn unchanged(
    call: &SourceCall,
    before: &Observation,
    after: &Observation,
) -> Result<()> {
    let old = settled(call, before)?;
    let current = settled(call, after)?;
    ensure!(
        old.journal == current.journal
            && serde_json::to_value(&old.inspection_after)?
                == serde_json::to_value(&current.inspection_after)?,
        "admission changed original source journal or physical reservations"
    );
    for field in [
        "id",
        "pinned_deployment_id",
        "pinned_service_protocol_version",
        "retry_count",
        "journal_size",
        "completion_result",
    ] {
        ensure!(
            observe::status(&before.after, &call.child_id)?.get(field)
                == observe::status(&after.after, &call.child_id)?.get(field),
            "admission changed original source {field}"
        );
    }
    Ok(())
}

pub(super) fn input_bytes(call: &SourceCall, observation: &Observation) -> Result<Vec<u8>> {
    let child = settled(call, observation)?;
    let first = super::super::super::http::rows(&child.journal)?
        .first()
        .context("original source input absent")?;
    let entry = journal::decode(
        first
            .get("entry_json")
            .context("original source input JSON absent")?,
    )?;
    journal::bytes(
        entry
            .pointer("/Command/Input/payload")
            .context("original source input payload absent")?,
    )
}

fn settled<'a>(call: &SourceCall, current: &'a Observation) -> Result<&'a Child> {
    let status = observe::status(&current.after, &call.child_id)?;
    ensure!(
        input::text(status, "status")? == "completed"
            && input::text(status, "completion_result")? == "success",
        "original source is not natively settled"
    );
    let child = current
        .children
        .iter()
        .find(|child| child.id == call.child_id && child.key == call.child_key)
        .context("original source observation absent")?;
    ensure!(
        matches!(
            child.inspection_after,
            TeamsSourceInspection::Settled { .. }
        ) && serde_json::to_value(&child.inspection_before)?
            == serde_json::to_value(&child.inspection_after)?,
        "original source inspection is not stable and settled"
    );
    let entries = journal::complete(&child.journal, status, &call.child_id)?
        .context("original settled source journal incomplete")?;
    journal::child_input(entries, call)?;
    super::super::boundary::validate_progress(super::super::recovery::progress(
        &child.inspection_after,
    ))?;
    Ok(child)
}
