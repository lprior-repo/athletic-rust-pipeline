use super::shape;
use anyhow::{ensure, Context, Result};
use serde_json::Value;

pub(super) fn completion_ids(future: &Value) -> Result<[Option<u32>; 256]> {
    let mut pending = Vec::new();
    pending.try_reserve_exact(256)?;
    pending.push(future);
    let mut ids = [None; 256];
    (0..256).try_for_each(|index| {
        let Some(node) = pending.pop() else {
            return Ok::<_, anyhow::Error>(());
        };
        *ids.get_mut(index)
            .context("future index exceeds witness budget")? = visit(node, &mut pending)?;
        Ok(())
    })?;
    ensure!(
        pending.is_empty(),
        "suspended future traversal exceeds witness budget"
    );
    Ok(ids)
}

fn visit<'a>(node: &'a Value, pending: &mut Vec<&'a Value>) -> Result<Option<u32>> {
    let branch = node
        .as_object()
        .context("suspended future node malformed")?;
    ensure!(branch.len() == 1, "ambiguous suspended future node");
    let (kind, values) = branch
        .iter()
        .next()
        .context("suspended future variant absent")?;
    if kind == "Single" {
        return notification(values);
    }
    ensure!(
        [
            "FirstCompleted",
            "AllCompleted",
            "FirstSucceededOrAllFailed",
            "AllSucceededOrFirstFailed",
            "Unknown"
        ]
        .contains(&kind.as_str()),
        "unknown suspended future variant"
    );
    let children = values.as_array().context("future branch is not an array")?;
    ensure!(
        pending
            .len()
            .checked_add(children.len())
            .is_some_and(|size| size <= 256),
        "suspended future exceeds witness budget"
    );
    pending.extend(children);
    Ok(None)
}

fn notification(value: &Value) -> Result<Option<u32>> {
    let leaf = value.as_object().context("future notification malformed")?;
    ensure!(leaf.len() == 1, "ambiguous future notification");
    let (kind, payload) = leaf
        .iter()
        .next()
        .context("future notification variant absent")?;
    match kind.as_str() {
        "CompletionId" | "SignalIndex" => {
            let id = shape::integer(
                payload,
                u64::from(u32::MAX),
                "notification index outside u32",
            )?;
            Ok((kind == "CompletionId").then_some(u32::try_from(id)?))
        }
        "SignalName" => {
            ensure!(payload.is_string(), "signal name malformed");
            Ok(None)
        }
        _ => anyhow::bail!("unknown future notification"),
    }
}
