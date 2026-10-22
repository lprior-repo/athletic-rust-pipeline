use super::super::input;
use super::{decode, variant};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;

pub(in super::super) fn awaited(status: &Value) -> Result<BTreeSet<u32>> {
    parse(status, false)
}

pub(in super::super) fn awaited_child(status: &Value) -> Result<BTreeSet<u32>> {
    parse(status, true)
}

fn parse(status: &Value, child_only: bool) -> Result<BTreeSet<u32>> {
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
    let (root, nodes) = if child_only {
        child_future(&future)?
    } else {
        (future.as_ref(), 256)
    };
    let mut pending = vec![root];
    let mut ids = BTreeSet::new();
    (0..nodes).try_for_each(|_| -> Result<()> {
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
            ensure!(
                !child_only || kind == "CompletionId",
                "child wait contains a signal"
            );
            if kind == "CompletionId" {
                let inserted = ids.insert(u32::try_from(
                    value.as_u64().context("future completion malformed")?,
                )?);
                ensure!(
                    !child_only || inserted,
                    "child wait duplicates a completion"
                );
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
        ensure!(
            !child_only || kind != "Unknown",
            "unknown child-wait branch"
        );
        let children = value.as_array().context("future branch malformed")?;
        ensure!(
            !child_only || !children.is_empty(),
            "empty child-wait branch"
        );
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

fn child_future(future: &Value) -> Result<(&Value, usize)> {
    let (kind, value) = variant(future)?;
    if kind != "FirstCompleted" {
        return Ok((future, 256));
    }
    let children = value.as_array().context("future branch malformed")?;
    let [first, second] = children.as_slice() else {
        return Ok((future, 256));
    };
    Ok(match (cancellation(first), cancellation(second)) {
        (true, false) => (second, 254),
        (false, true) => (first, 254),
        _ => (future, 256),
    })
}

fn cancellation(future: &Value) -> bool {
    future.as_object().is_some_and(|outer| {
        outer.len() == 1
            && outer
                .get("Single")
                .and_then(Value::as_object)
                .is_some_and(|notification| {
                    notification.len() == 1
                        && notification.get("SignalIndex").and_then(Value::as_u64) == Some(1)
                })
    })
}
