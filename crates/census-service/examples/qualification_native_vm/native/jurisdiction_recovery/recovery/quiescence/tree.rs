use super::super::super::super::http::{query, rows};
use super::super::super::input::{self, Original};
use super::Pause;
use anyhow::{ensure, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;

const MAX_INVOCATIONS: usize = 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct TreePause {
    pub(in super::super) original: Pause,
    pub(in super::super) descendant_pauses: Vec<Pause>,
    pub(in super::super) before: Vec<Value>,
    pub(in super::super) after: Vec<Value>,
}

impl TreePause {
    pub(in super::super) fn verify(&self, original: &Original) -> Result<()> {
        ensure!(
            self.original.invocation_id == original.id && self.original.status_code == 202,
            "pause acknowledgement does not bind original invocation"
        );
        ensure!(
            !self.before.is_empty()
                && self.before.len() <= MAX_INVOCATIONS
                && self.after.len() <= MAX_INVOCATIONS
                && self.descendant_pauses.len() <= MAX_INVOCATIONS,
            "pause tree exceeds budget or is absent"
        );
        ensure!(
            self.before.iter().all(|old| self
                .after
                .iter()
                .any(|row| row.get("id") == old.get("id")
                    && row.get("target_service_key") == old.get("target_service_key")
                    && row.get("invoked_by_id") == old.get("invoked_by_id"))),
            "paused tree lost or changed an admitted invocation"
        );
        ensure!(
            self.after.iter().all(quiet),
            "paused tree retains active descendants"
        );
        ensure!(
            self.after
                .iter()
                .any(
                    |row| row.get("id").and_then(Value::as_str) == Some(&original.id)
                        && row.get("status").and_then(Value::as_str) == Some("paused")
                ),
            "original root is not paused in tree witness"
        );
        ensure!(
            self.descendant_pauses.iter().all(|pause| {
                pause.status_code == 202
                    && self.after.iter().any(|row| {
                        row.get("id").and_then(Value::as_str) == Some(&pause.invocation_id)
                    })
            }),
            "descendant pause is unacknowledged or unbound"
        );
        Ok(())
    }
}

enum Progress {
    Waiting(Vec<Pause>),
    Quiet {
        pauses: Vec<Pause>,
        after: Vec<Value>,
    },
}

#[tracing::instrument(skip(client, original, pause))]
pub(in super::super) async fn settle(
    client: &Client,
    original: &Original,
    pause: Pause,
) -> Result<TreePause> {
    let before = inventory(client, original).await?;
    let mut descendant_pauses = Vec::new();
    descendant_pauses.try_reserve_exact(MAX_INVOCATIONS)?;
    let progress = stream::iter(0..120u32)
        .map(Ok::<_, anyhow::Error>)
        .try_fold(
            Progress::Waiting(descendant_pauses),
            |progress, _| async move {
                let Progress::Waiting(pauses) = progress else {
                    return Ok(progress);
                };
                let current = inventory(client, original).await?;
                let pauses = pause_active(client, original, &current, pauses).await?;
                let after = inventory(client, original).await?;
                if current == after && after.iter().all(quiet) {
                    return Ok(Progress::Quiet { pauses, after });
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(Progress::Waiting(pauses))
            },
        )
        .await?;
    let Progress::Quiet {
        pauses: descendant_pauses,
        after,
    } = progress
    else {
        return Err(anyhow::anyhow!(
            "original invocation tree did not become quiescent"
        ));
    };
    let proof = TreePause {
        original: pause,
        descendant_pauses,
        before,
        after,
    };
    proof.verify(original)?;
    Ok(proof)
}

#[tracing::instrument(skip(client, original, current, pauses))]
async fn pause_active(
    client: &Client,
    original: &Original,
    current: &[Value],
    pauses: Vec<Pause>,
) -> Result<Vec<Pause>> {
    stream::iter(current)
        .map(Ok::<_, anyhow::Error>)
        .try_fold(pauses, |mut pauses, row| async move {
            let id = input::text(row, "id")?;
            if id != original.id
                && !quiet(row)
                && !pauses.iter().any(|pause| pause.invocation_id == id)
            {
                ensure!(
                    pauses.len() < MAX_INVOCATIONS,
                    "descendant pause budget exhausted"
                );
                pauses.push(super::pause_id(client, id).await?);
            }
            Ok(pauses)
        })
        .await
}

fn quiet(row: &Value) -> bool {
    matches!(
        row.get("status").and_then(Value::as_str),
        Some("paused" | "completed")
    )
}

#[tracing::instrument(skip(client, original))]
pub(in super::super) async fn inventory(
    client: &Client,
    original: &Original,
) -> Result<Vec<Value>> {
    let value = query(client, "SELECT id, target_service_name, target_service_key, target_handler_name, status, invoked_by_id, pinned_deployment_id, pinned_service_protocol_version, journal_size, completion_result, completion_failure FROM sys_invocation ORDER BY id LIMIT 1025").await?;
    let all = rows(&value)?;
    ensure!(
        all.len() <= MAX_INVOCATIONS,
        "owned native invocation inventory exceeds budget"
    );
    let mut ids = HashSet::new();
    ids.try_reserve(MAX_INVOCATIONS)?;
    ids.insert(original.id.as_str());
    for _ in 0..MAX_INVOCATIONS {
        let before = ids.len();
        all.iter().try_for_each(|row| -> Result<()> {
            let id = input::text(row, "id")?;
            input::safe_id(id)?;
            if row
                .get("invoked_by_id")
                .and_then(Value::as_str)
                .is_some_and(|parent| ids.contains(parent))
            {
                ids.insert(id);
            }
            Ok(())
        })?;
        if ids.len() == before {
            break;
        }
    }
    let mut tree = Vec::new();
    tree.try_reserve_exact(ids.len())?;
    tree.extend(
        all.iter()
            .filter(|row| {
                row.get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| ids.contains(id))
            })
            .cloned(),
    );
    ensure!(
        tree.iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(&original.id)),
        "original root absent from native invocation inventory"
    );
    Ok(tree)
}
