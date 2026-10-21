use super::{query, rows, safe_id, state, target, text};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::JurisdictionRequest;
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

mod call;
#[cfg(test)]
mod real_journal_tests;
#[cfg(test)]
mod tests;

pub(super) struct Expected {
    child: String,
    service: &'static str,
    key: String,
    input: Value,
}

impl Expected {
    pub(super) fn national(child: &str, input: &JurisdictionRequest) -> Result<Self> {
        safe_id(child)?;
        Ok(Self {
            child: child.to_owned(),
            service: "JurisdictionCensus",
            key: census_reconcile::identity::WorkflowIdentity::jurisdiction(
                input.jurisdiction,
                input.season,
                input.revision,
            )
            .as_str()
            .to_owned(),
            input: serde_json::to_value(input)?,
        })
    }

    pub(super) fn source(call: &super::super::super::super::journal::SourceCall) -> Result<Self> {
        safe_id(&call.child_id)?;
        Ok(Self {
            child: call.child_id.clone(),
            service: "TeamsSource",
            key: call.child_key.clone(),
            input: serde_json::to_value(&call.request)?,
        })
    }
}

pub(super) async fn suspended(
    client: &Client,
    id: &str,
    key: &str,
    expected: &Expected,
) -> Result<Value> {
    let service = if expected.service == "JurisdictionCensus" {
        "NationalCensus"
    } else {
        "JurisdictionCensus"
    };
    let probes = stream::iter(0..120u32)
        .then(|_| async move {
            let before = state(client, id).await?;
            target(&before, id, key, service)?;
            ensure!(
                matches!(text(&before, "status")?, "running" | "ready" | "suspended"),
                "parent escaped the bounded child wait"
            );
            if text(&before, "status")? == "suspended" {
                let journal = journal(client, id).await?;
                let after = state(client, id).await?;
                target(&after, id, key, service)?;
                if before == after {
                    let proof = call::verify(&before, &journal, expected)?;
                    return Ok(Some(
                        json!({"state":before,"journal":journal,"child_wait":proof}),
                    ));
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<_, anyhow::Error>(None)
        })
        .try_filter_map(|proof| futures::future::ready(Ok(proof)));
    futures::pin_mut!(probes);
    tokio::time::timeout(Duration::from_secs(6), probes.try_next()).await
        .context("exact child-wait six-second deadline expired")??
        .context("original parent did not suspend on its exact unfinished child within six-second observation window")
}

pub(super) async fn journal(client: &Client, id: &str) -> Result<Value> {
    safe_id(id)?;
    let value = query(client, &format!("SELECT id, index, version, entry_type, entry_json FROM sys_journal WHERE id = '{id}' ORDER BY index LIMIT 2049")).await?;
    ensure!(
        rows(&value)?.len() <= 2048,
        "control journal exceeds 2048 entries"
    );
    Ok(value)
}

pub(super) fn prefix(before: &Value, after: &Value) -> Result<()> {
    let original = rows(before)?;
    let retained = rows(after)?;
    ensure!(
        retained.get(..original.len()) == Some(original),
        "native control rewrote original journal prefix"
    );
    Ok(())
}

pub(super) fn complete(status: &Value, retained: &Value, id: &str) -> Result<()> {
    super::super::super::super::journal::complete(retained, status, id)?
        .context("control baseline journal incomplete")?;
    Ok(())
}
