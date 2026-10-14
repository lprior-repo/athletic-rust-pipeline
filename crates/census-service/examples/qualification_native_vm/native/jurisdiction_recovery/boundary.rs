use super::injection;
use super::input::{self, Original};
use super::journal::{self, SourceCall};
use super::observe::{self, Observation};
use anyhow::{ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

mod assess;
mod identity;
mod witness;

pub(super) use identity::{
    identity, journal_started, pinned_identity, target_identity, validate_progress,
};

#[tracing::instrument(skip(client, original))]
pub(super) async fn wait(client: &Client, original: &Original) -> Result<Value> {
    let probes = stream::iter(0..120_u32).then(|attempt| async move {
        let observation = observe::read(client, original).await?;
        let witness = runtime_assess(&observation, original);
        let candidate = match witness {
            Ok(witness) => witness,
            Err(error) => {
                observe::publish("invalid-boundary", &json!({"attempt":attempt,"observation":observation,"error":format!("{error:#}")}))?;
                return Err(error);
            }
        };
        observe::publish("latest-probe", &json!({"attempt":attempt,"observation":observation,"witness":candidate}))?;
        if let Some(candidate) = candidate.as_ref() {
            match confirm(client, original, candidate).await {
                Ok(Some(confirmed)) => return Ok(Some(confirmed)),
                Ok(None) => {}
                Err(error) => {
                    observe::publish("invalid-boundary", &json!({"attempt":attempt,"observation":observation,"error":format!("{error:#}")}))?;
                    return Err(error);
                }
            }
        }
        let completed = observe::find_status(&observation.after, &original.id)?
            .map(|parent| input::text(parent, "status").map(|status| status == "completed"))
            .transpose()?.is_some_and(|completed| completed);
        if completed {
            let error = "original completed before fully published native reservation marker and active source boundary; reset refused";
            observe::publish("invalid-boundary", &json!({"attempt":attempt,"observation":observation,"error":error}))?;
            anyhow::bail!(error);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        Ok::<_, anyhow::Error>(None)
    }).try_filter_map(|value| futures::future::ready(Ok(value)));
    futures::pin_mut!(probes);
    let reached = probes.try_next().await?;
    if reached.is_none() {
        observe::publish(
            "invalid-boundary",
            &json!({"error":"no fully published matching reservation marker and active original source witness reached in 120 bounded probes; QMP reset refused","operation":injection::operation(original)?,"latest_probe":"jurisdiction-recovery-latest-probe.json"}),
        )?;
    }
    reached.context("native reservation marker missing, mismatched or unreached at the active original source boundary; QMP reset refused")
}

#[tracing::instrument(skip(client, original))]
async fn confirm(client: &Client, original: &Original, initial: &Value) -> Result<Option<Value>> {
    let captures = super::captures::snapshot(original.key.clone()).await?;
    let observation = observe::read(client, original).await?;
    let witness = runtime_assess(&observation, original)?;
    if let Some(current) = witness.as_ref() {
        ensure!(
            initial.get("native_source_boundary") == current.get("native_source_boundary"),
            "native reservation marker/config changed during boundary confirmation"
        );
    }
    Ok(witness
        .map(|witness| json!({"observation":observation,"witness":witness,"captures":captures})))
}

fn runtime_assess(observation: &Observation, original: &Original) -> Result<Option<Value>> {
    let operation = injection::operation(original)?;
    assess::assess_source(observation, original, Some(&operation))?
        .map(|witness| injection::witness(observation, original, witness))
        .transpose()
        .map(Option::flatten)
}

#[cfg(test)]
pub(super) fn assess(observation: &Observation, original: &Original) -> Result<Option<Value>> {
    assess::assess_source(observation, original, None)
}

#[cfg(test)]
mod tests;
