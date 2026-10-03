use super::super::{input, journal};
use super::LIMIT;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Registration {
    pub(super) request_digest: String,
    pub(super) observed_on: String,
}

pub(super) fn registration(entries: &[Value]) -> Result<Option<(Registration, Value)>> {
    let Some(command) = entries.get(1) else {
        return Ok(None);
    };
    let entry = journal::decode(
        command
            .get("entry_json")
            .context("registration command JSON absent")?,
    )?;
    let run = entry
        .pointer("/Command/Run")
        .context("first source effect is not registration Run")?;
    ensure!(
        input::text(command, "entry_type")? == "Command: Run",
        "registration command type differs"
    );
    let completion = run
        .get("completion_id")
        .and_then(Value::as_u64)
        .context("registration completion ID absent")?;
    ensure!(
        u32::try_from(completion).is_ok(),
        "registration completion ID outside protocol budget"
    );
    let found = entries.iter().skip(2).try_fold(None, |found, row| -> Result<_> {
        let entry = journal::decode(row.get("entry_json").context("registration journal JSON absent")?)?;
        let Some(run) = entry.pointer("/Notification/Completion/Run") else { return Ok(found); };
        if run.get("completion_id").and_then(Value::as_u64) != Some(completion) {
            return Ok(found);
        }
        ensure!(found.is_none(), "duplicate journaled source registration completion");
        ensure!(input::text(row, "entry_type")? == "Notification: Run", "registration notification type differs");
        let bytes = success(run)?;
        let identity: Registration = serde_json::from_slice(&bytes).context("journaled source registration malformed")?;
        validate(&identity)?;
        Ok(Some((identity, json!({"command":journal::reference(command)?,"completion":journal::reference(row)?}))))
    })?;
    Ok(found)
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
        u64::try_from(bytes.len())? <= LIMIT,
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
