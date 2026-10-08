use super::super::input::{self, Original};
use super::super::observe::{self, Observation};
use anyhow::{ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration;

mod completion;
pub(super) use completion::verify_tree as verify_completed_tree;

mod tree;
pub(super) use tree::{inventory, TreePause};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pause {
    pub(super) invocation_id: String,
    pub(super) status_code: u16,
    pub(super) response_bytes: usize,
    pub(super) response_sha256: String,
}

pub(super) enum Control {
    Completed,
    Paused(TreePause),
}

#[tracing::instrument(skip(client, original, candidate))]
pub(super) async fn hold(
    client: &Client,
    original: &Original,
    candidate: Observation,
) -> Result<(Observation, Control)> {
    if let Some(completed) = completion::read(client, original, &candidate).await? {
        return Ok((completed, Control::Completed));
    }
    let pause = pause_id(client, &original.id).await?;
    if let Some(completed) = completion::read(client, original, &candidate).await? {
        return Ok((completed, Control::Completed));
    }
    ensure!(pause.status_code == 202, "original invocation pause was not accepted: {}", pause.status_code);
    let Some(tree) = tree::settle(client, original, pause).await? else {
        let completed = completion::read(client, original, &candidate).await?
            .context("completed original tree did not yield fresh coherent success evidence")?;
        return Ok((completed, Control::Completed));
    };
    let checks = stream::iter(0..120u32)
        .then(|_| async {
            let observation = observe::read(client, original).await?;
            let parent = observe::status(&observation.after, &original.id)?;
            if input::text(parent, "status")? == "completed" {
                if let Some(completed) = completion::read(client, original, &candidate).await? {
                    return Ok(Some((completed, true)));
                }
            } else if input::text(parent, "status")? == "paused"
                && super::candidate(original, &observation)?
            {
                return Ok(Some((observation, false)));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok::<_, anyhow::Error>(None)
        })
        .try_filter_map(|value| futures::future::ready(Ok(value)));
    futures::pin_mut!(checks);
    let (observation, completed) = checks
        .try_next()
        .await?
        .context("original pause did not yield coherent settled source evidence")?;
    Ok((observation, if completed { Control::Completed } else { Control::Paused(tree) }))
}

#[tracing::instrument(skip(client))]
async fn pause_id(client: &Client, id: &str) -> Result<Pause> {
    input::safe_id(id)?;
    let response = client
        .patch(format!(
            "{}invocations/{id}/pause",
            super::super::super::ADMIN
        ))
        .send()
        .await?;
    let status_code = response.status().as_u16();
    let bytes = response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |mut bytes, part| async move {
            ensure!(
                bytes
                    .len()
                    .checked_add(part.len())
                    .is_some_and(|total| total <= 4096),
                "pause acknowledgement exceeds four KiB"
            );
            bytes.try_reserve(part.len())?;
            bytes.extend_from_slice(&part);
            Ok(bytes)
        })
        .await?;
    let pause = Pause {
        invocation_id: id.to_string(),
        status_code,
        response_bytes: bytes.len(),
        response_sha256: format!("{:x}", Sha256::digest(&bytes)),
    };
    super::super::super::super::artifacts::append(
        &std::path::Path::new(super::super::super::super::GUEST)
            .join("invocation-pause-acknowledgements.jsonl"),
        &json!({"pause":pause,"response_body_utf8":String::from_utf8_lossy(&bytes)}),
    )?;
    Ok(pause)
}
