use super::super::input;
use super::{decode, variant};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;

pub(in super::super) fn awaited(status: &Value) -> Result<BTreeSet<u32>> {
    let field = match input::text(status, "status")? {
        "suspended" => "suspended_waiting_future_json",
        "running" => "last_awaiting_on_future_json",
        _ => anyhow::bail!("source parent is neither running nor suspended"),
    };
    let future = status
        .get(field)
        .with_context(|| format!("required {field} absent"))?;
    if future.is_null() {
        return Ok(BTreeSet::new());
    }
    let future = decode(future)?;
    let mut pending = vec![future.as_ref()];
    let mut ids = BTreeSet::new();
    (0..256).try_for_each(|_| -> Result<()> {
        let Some(node) = pending.pop() else {
            return Ok(());
        };
        let (kind, value) = variant(node)?;
        if kind == "Single" {
            let (kind, value) = variant(value)?;
            ensure!(
                matches!(kind, "CompletionId" | "SignalIndex" | "SignalName"),
                "unknown future notification"
            );
            if kind == "CompletionId" {
                ids.insert(u32::try_from(
                    value.as_u64().context("future completion malformed")?,
                )?);
            }
            return Ok(());
        }
        ensure!(
            [
                "FirstCompleted",
                "AllCompleted",
                "FirstSucceededOrAllFailed",
                "AllSucceededOrFirstFailed",
                "Unknown"
            ]
            .contains(&kind),
            "unknown future branch"
        );
        let children = value.as_array().context("future branch malformed")?;
        ensure!(
            pending
                .len()
                .checked_add(children.len())
                .is_some_and(|size| size <= 256),
            "source future traversal exceeds budget"
        );
        pending.try_reserve(children.len())?;
        pending.extend(children);
        Ok(())
    })?;
    ensure!(pending.is_empty(), "source future exceeds 256-node budget");
    Ok(ids)
}
