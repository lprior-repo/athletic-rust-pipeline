use super::super::super::input::{self, Original};
use super::super::super::observe::{self, Observation};
use super::super::{candidate, reconcile, source_reconciliation};
use super::tree;
use anyhow::{ensure, Context, Result};
use reqwest::Client;
use serde_json::Value;

#[tracing::instrument(skip(client, original, previous))]
pub(super) async fn read(
    client: &Client,
    original: &Original,
    previous: &Observation,
) -> Result<Option<Observation>> {
    let before = tree::inventory(client, original).await?;
    let current = observe::read(client, original).await?;
    let after = tree::inventory(client, original).await?;
    Ok(admit(original, previous, &current, &before, &after)?.then_some(current))
}

pub(super) fn admit(
    original: &Original,
    previous: &Observation,
    current: &Observation,
    before: &[Value],
    after: &[Value],
) -> Result<bool> {
    let parent = observe::status(&current.after, &original.id)?;
    if input::text(parent, "status")? != "completed" {
        return Ok(false);
    }
    ensure!(
        input::text(parent, "completion_result")? == "success",
        "original completed with terminal failure"
    );
    if before != after || !candidate(original, current)? {
        return Ok(false);
    }
    verify_tree(original, after)?;
    bind_observation(current, after)?;
    reconcile::retained_observation(original, previous, current)?;
    source_reconciliation(original, current)?;
    Ok(true)
}

pub(in super::super) fn verify_tree(original: &Original, tree: &[Value]) -> Result<()> {
    ensure!(
        !tree.is_empty() && tree.len() <= 1024,
        "completed original tree absent or exceeds budget"
    );
    let parent = tree
        .iter()
        .find(|row| row.get("id").and_then(Value::as_str) == Some(&original.id))
        .context("completed original root absent from tree")?;
    super::super::super::boundary::identity(
        parent,
        &original.id,
        &original.key,
        "JurisdictionCensus",
        original,
    )?;
    tree.iter()
        .enumerate()
        .try_for_each(|(index, row)| -> Result<()> {
            let id = input::text(row, "id")?;
            input::safe_id(id)?;
            ensure!(
                !tree
                    .iter()
                    .take(index)
                    .any(|prior| prior.get("id") == row.get("id")),
                "duplicate completed descendant identity"
            );
            ensure!(
                input::text(row, "status")? == "completed"
                    && input::text(row, "completion_result")? == "success",
                "completed original retains active or failed descendant work"
            );
            if id != original.id {
                let owner = input::text(row, "invoked_by_id")?;
                ensure!(
                    tree.iter()
                        .any(|parent| parent.get("id").and_then(Value::as_str) == Some(owner)),
                    "completed descendant has no retained owner"
                );
            }
            Ok(())
        })
}

fn bind_observation(observation: &Observation, tree: &[Value]) -> Result<()> {
    super::super::super::super::http::rows(&observation.after)?
        .iter()
        .try_for_each(|row| -> Result<()> {
            let retained = tree
                .iter()
                .find(|retained| retained.get("id") == row.get("id"))
                .context("completed original observation missing from actual tree")?;
            ensure!(
                [
                    "target_service_name",
                    "target_service_key",
                    "target_handler_name",
                    "status",
                    "invoked_by_id",
                    "pinned_deployment_id",
                    "pinned_service_protocol_version",
                    "journal_size",
                    "completion_result",
                ]
                .into_iter()
                .all(|field| retained.get(field) == row.get(field)),
                "completed actual tree differs from original observation"
            );
            Ok(())
        })
}

#[cfg(test)]
mod tests;
