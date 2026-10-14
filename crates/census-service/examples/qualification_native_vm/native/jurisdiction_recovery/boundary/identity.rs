use super::input::{self, Original};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::TeamsAttemptProgress;
use serde_json::Value;

pub(in super::super) fn journal_started(status: &Value) -> Result<bool> {
    match status.get("journal_size") {
        None | Some(Value::Null) => Ok(false),
        Some(size) => Ok(size.as_u64().context("journal size malformed")? > 0),
    }
}

pub(in super::super) fn identity(
    status: &Value,
    id: &str,
    key: &str,
    service: &str,
    original: &Original,
) -> Result<()> {
    target_identity(status, id, key, service)?;
    pinned_identity(status, input::text(&original.deployment, "id")?)
        .with_context(|| format!("original {service} invocation {id} pin evidence: {status}"))
}

pub(in super::super) fn target_identity(
    status: &Value,
    id: &str,
    key: &str,
    service: &str,
) -> Result<()> {
    ensure!(
        input::text(status, "id")? == id
            && input::text(status, "target_service_key")? == key
            && input::text(status, "target_service_name")? == service
            && input::text(status, "target_handler_name")? == "run",
        "original invocation target identity mismatch"
    );
    ensure!(
        status.get("restarted_from").is_none_or(Value::is_null),
        "replacement invocation cannot prove original recovery"
    );
    Ok(())
}

pub(in super::super) fn pinned_identity(status: &Value, deployment: &str) -> Result<()> {
    ensure!(
        input::text(status, "pinned_deployment_id")? == deployment,
        "source invocation deployment is not the retained deployment"
    );
    ensure!(
        status
            .get("pinned_service_protocol_version")
            .and_then(Value::as_u64)
            .is_some_and(|version| (4..=7).contains(&version)),
        "source invocation protocol outside journal-v2 scope"
    );
    Ok(())
}

pub(in super::super) fn validate_progress(progress: &[TeamsAttemptProgress]) -> Result<()> {
    ensure!(
        progress.len() <= 3,
        "source physical-attempt ceiling exceeded"
    );
    progress
        .iter()
        .enumerate()
        .try_for_each(|(index, step)| -> Result<()> {
            let attempt = match step {
                TeamsAttemptProgress::Unknown { attempt }
                | TeamsAttemptProgress::Completed { attempt, .. }
                | TeamsAttemptProgress::Transient { attempt, .. }
                | TeamsAttemptProgress::Terminal { attempt, .. } => *attempt,
            };
            ensure!(
                usize::from(attempt) == index.checked_add(1).context("source ordinal overflow")?,
                "source reservation ordinals changed"
            );
            Ok(())
        })
}
