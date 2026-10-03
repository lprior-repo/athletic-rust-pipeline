use super::artifacts;
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) mod snapshot;

pub fn payload_digest(payload: &Value) -> Result<String> {
    let table = payload
        .get("table")
        .and_then(Value::as_str)
        .context("payload table absent")?;
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .context("payload rows absent")?;
    let mut hash = Sha256::new();
    hash.update(table.as_bytes());
    rows.iter().try_for_each(|row| -> Result<()> {
        hash.update(serde_json::to_vec(row)?);
        Ok(())
    })?;
    Ok(format!("{:x}", hash.finalize()))
}

fn equal_rows(expected: &[Value], physical: &[Value]) -> Result<()> {
    let canonical = |rows: &[Value]| -> Result<Vec<String>> {
        let mut encoded = rows
            .iter()
            .map(serde_json::to_string)
            .collect::<serde_json::Result<Vec<_>>>()?;
        encoded.sort_unstable();
        Ok(encoded)
    };
    ensure!(
        canonical(expected)? == canonical(physical)?,
        "exact physical observations differ: lost, changed or duplicated source rows"
    );
    Ok(())
}

fn capture(root: &Path, name: &str) -> Result<Value> {
    let original = artifacts::json(&root.join(format!("{name}.meta.json")))?;
    let url = original
        .get("url")
        .and_then(Value::as_str)
        .context("original source URL absent")?;
    let hash = artifacts::sha(format!("GET\u{1f}{url}\u{1f}").as_bytes());
    let key = hash.get(..32).context("capture key absent")?;
    let metadata = artifacts::json(&root.join(format!("store/http/{key}.meta.json")))?;
    let body = artifacts::read(&root.join(format!("store/http/{key}.body")))?;
    ensure!(
        original == metadata,
        "immutable capture metadata changed across reboot/clock"
    );
    ensure!(
        body == artifacts::read(&root.join(format!("{name}.body")))?,
        "immutable capture bytes changed"
    );
    ensure!(
        metadata.get("content_digest").and_then(Value::as_str)
            == Some(artifacts::sha(&body).as_str()),
        "physical capture digest mismatch"
    );
    Ok(
        json!({"metadata":metadata,"sha256":artifacts::sha(&body),"bytes":body.len(),"fresh_public_acquisition":false}),
    )
}

pub fn reconcile(ack: &Value, reboot: &Value, clock: &Value) -> Result<Value> {
    ensure!(
        reboot.get("receipt") == clock.get("receipt"),
        "receipt identity/date/content changed at midnight"
    );
    [
        "physical_rows",
        "tables_digest",
        "captures",
        "manifest",
        "deployment",
    ]
    .into_iter()
    .try_for_each(|key| -> Result<()> {
        ensure!(
            reboot.get(key).is_some() && reboot.get(key) == clock.get(key),
            "physical recovery/clock invariant changed: {key}"
        );
        Ok(())
    })?;
    ensure!(
        reboot.get("acknowledgement") == Some(ack),
        "guest receipt evidence disagrees with host-retained acknowledgement"
    );
    Ok(
        json!({"verdict":"PASS","scope":"exact acknowledged physical rows, effect receipt, immutable capture bodies/metadata and deployment survival; same-effect replay across guest midnight","fresh_acquisition":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_duplicate_is_not_hidden_by_merged_entity_identity() -> Result<()> {
        let row = json!({"id":"source-owned", "date":"2026-10-01"});
        equal_rows(std::slice::from_ref(&row), std::slice::from_ref(&row))?;
        ensure!(
            equal_rows(&[row.clone()], &[row.clone(), row]).is_err(),
            "duplicated physical observation was accepted"
        );
        Ok(())
    }

    #[test]
    fn original_capture_and_receipt_dates_cannot_be_replaced() -> Result<()> {
        let base = json!({"receipt":{"at":"2026-10-01"},"physical_rows":[],"tables_digest":"a","captures":[],"manifest":{},"deployment":{},"acknowledgement":{}});
        let mut changed = base.clone();
        *changed
            .get_mut("receipt")
            .and_then(|receipt| receipt.get_mut("at"))
            .context("test receipt date absent")? = json!("2026-10-02");
        reconcile(&json!({}), &base, &base)?;
        ensure!(
            reconcile(&json!({}), &base, &changed).is_err(),
            "changed durable receipt date was accepted"
        );
        Ok(())
    }
}
