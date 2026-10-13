use super::super::{input, journal};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const REGISTRATION_LIMIT: usize = 4096;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Registration {
    pub(super) request_digest: String,
    pub(super) observed_on: String,
}

pub(super) fn registration(entries: &[Value]) -> Result<Option<(Registration, Value)>> {
    let mut resolved = None;
    for command in entries.iter().skip(1) {
        let Ok(entry) = journal::decode(
            command
                .get("entry_json")
                .context("registration command JSON absent")?,
        ) else {
            continue;
        };
        let Some(run) = entry.pointer("/Command/Run") else {
            continue;
        };
        if input::text(command, "entry_type")? != "Command: Run" {
            continue;
        }
        let Some(completion) = run.get("completion_id").and_then(Value::as_u64) else {
            continue;
        };
        ensure!(
            u32::try_from(completion).is_ok(),
            "registration completion ID outside protocol budget"
        );
        let mut candidate = None;
        for row in entries.iter().skip(1) {
            let entry = journal::decode(
                row.get("entry_json")
                    .context("registration journal JSON absent")?,
            )?;
            let Some(run) = entry.pointer("/Notification/Completion/Run") else {
                continue;
            };
            if run.get("completion_id").and_then(Value::as_u64) != Some(completion) {
                continue;
            }
            ensure!(
                input::text(row, "entry_type")? == "Notification: Run",
                "registration notification type differs"
            );
            let bytes = success(run)?;
            if bytes == b"null" {
                continue;
            }
            let identity: Registration = serde_json::from_slice(&bytes)
                .context("journaled source registration malformed")?;
            validate(&identity)?;
            ensure!(
                candidate.is_none(),
                "duplicate journaled source registration completion"
            );
            candidate = Some((
                identity,
                json!({"command":journal::reference(command)?,"completion":journal::reference(row)?}),
            ));
        }
        if let Some(pair) = candidate {
            ensure!(
                resolved.is_none(),
                "duplicate journaled source registration"
            );
            resolved = Some(pair);
        }
    }
    Ok(resolved)
}

fn success(run: &Value) -> Result<Vec<u8>> {
    let result = run
        .get("result")
        .and_then(Value::as_object)
        .context("registration result malformed")?;
    ensure!(result.len() == 1, "ambiguous registration result");
    let bytes = result
        .get("Success")
        .and_then(Value::as_array)
        .context("registration did not journal success")?;
    ensure!(
        bytes.len() <= REGISTRATION_LIMIT,
        "registration identity exceeds 4096 bytes"
    );
    bytes
        .iter()
        .map(|byte| {
            Ok(u8::try_from(
                byte.as_u64()
                    .context("registration payload byte malformed")?,
            )?)
        })
        .collect()
}

fn validate(identity: &Registration) -> Result<()> {
    ensure!(
        identity.request_digest.len() == 64
            && identity
                .request_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
        "journaled registration request digest malformed"
    );
    let date = chrono::NaiveDate::parse_from_str(&identity.observed_on, "%Y-%m-%d")
        .context("journaled registration observation date malformed")?;
    ensure!(
        date.format("%Y-%m-%d").to_string() == identity.observed_on,
        "journaled registration date is not canonical"
    );
    Ok(())
}
