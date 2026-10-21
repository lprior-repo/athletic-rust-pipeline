use super::super::super::super::http::{query, rows};
use super::super::super::journal;
use super::super::{identity, safe_id, text};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::JurisdictionRequest;
use reqwest::Client;
use serde_json::Value;

pub(super) async fn verify(
    client: &Client,
    parent: &str,
    expected: &JurisdictionRequest,
    status: &Value,
) -> Result<Option<String>> {
    let Some(status) = rows(status)?.first() else {
        return Ok(None);
    };
    let child = text(status, "id")?;
    safe_id(child)?;
    ensure!(
        text(status, "invoked_by_id")? == parent
            && text(status, "target_service_name")? == "JurisdictionCensus"
            && text(status, "target_service_key")? == identity(expected)
            && text(status, "target_handler_name")? == "run",
        "national child target differs"
    );
    let retained = query(client, &format!(
        "SELECT id, index, version, entry_type, entry_json FROM sys_journal WHERE id = '{parent}' ORDER BY index LIMIT 257"
    )).await?;
    ensure!(
        rows(&retained)?.len() <= 256,
        "national preparation journal exceeds budget"
    );
    let calls = rows(&retained)?
        .iter()
        .try_fold(0usize, |count, row| -> Result<usize> {
            ensure!(text(row, "id")? == parent, "foreign national journal row");
            let entry = journal::decode(
                row.get("entry_json")
                    .context("national journal entry absent")?,
            )?;
            let Some(request) = entry.pointer("/Command/Call/request") else {
                return Ok(count);
            };
            let Some(target) = request.pointer("/invocation_target/VirtualObject") else {
                return Ok(count);
            };
            if text(target, "name")? != "JurisdictionCensus" {
                return Ok(count);
            }
            ensure!(
                text(target, "handler")? == "run"
                    && text(target, "handler_ty")? == "Exclusive"
                    && text(target, "key")? == identity(expected)
                    && text(request, "invocation_id")? == child,
                "national original child call differs"
            );
            let input: JurisdictionRequest = serde_json::from_slice(&journal::bytes(
                request
                    .get("parameter")
                    .context("national child input absent")?,
            )?)?;
            ensure!(
                serde_json::to_value(&input)? == serde_json::to_value(expected)?,
                "national actual child input differs from qualification scope"
            );
            ensure!(
                text(row, "entry_type")? == "Command: Call"
                    && row.get("version").and_then(Value::as_u64) == Some(2),
                "national call journal schema differs"
            );
            count
                .checked_add(1)
                .context("national child count overflow")
        })?;
    ensure!(calls <= 1, "duplicate original national child call");
    Ok((calls == 1).then(|| child.to_owned()))
}
