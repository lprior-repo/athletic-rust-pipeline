use super::artifacts;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    accepted: u64,
    completed: u64,
    cancelled: u64,
    timed_out: u64,
    aborted: u64,
    panicked: u64,
}

impl Counts {
    fn validate(&self) -> Result<()> {
        let terminal = self
            .completed
            .checked_add(self.cancelled)
            .context("drain terminal count overflow")?;
        ensure!(
            self.timed_out == 0 && self.aborted == 0 && self.panicked == 0,
            "drain contains timed-out, aborted or panicked tasks"
        );
        ensure!(
            self.accepted == terminal,
            "drain accepted count differs from completed plus cancelled"
        );
        Ok(())
    }
}

pub fn parse(log: &str) -> Result<Counts> {
    let mut lines = log
        .lines()
        .filter_map(|line| line.strip_prefix("drained: "));
    let line = lines
        .next()
        .context("current endpoint incarnation lacks production drain certificate")?;
    ensure!(
        lines.next().is_none(),
        "current endpoint emitted multiple drain certificates"
    );
    let mut fields = line.split_whitespace();
    let counts = Counts {
        accepted: field(&mut fields, "accepted=")?,
        completed: field(&mut fields, "completed=")?,
        cancelled: field(&mut fields, "cancelled=")?,
        timed_out: field(&mut fields, "timed_out=")?,
        aborted: field(&mut fields, "aborted=")?,
        panicked: field(&mut fields, "panicked=")?,
    };
    ensure!(
        fields.next().is_none(),
        "unexpected drain accounting fields"
    );
    counts.validate()?;
    Ok(counts)
}

fn field<'a>(fields: &mut impl Iterator<Item = &'a str>, prefix: &str) -> Result<u64> {
    let value = fields
        .next()
        .and_then(|field| field.strip_prefix(prefix))
        .context("missing or unordered drain accounting field")?;
    ensure!(
        !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
        "invalid drain count"
    );
    Ok(value.parse()?)
}

pub fn verify(root: &Path, boot: &str) -> Result<Value> {
    let owner = artifacts::json(&root.join("current-owner.json"))?;
    let certificate = artifacts::json(&root.join("current-drain.json"))?;
    verify_identity(&owner, &certificate, boot)?;
    let endpoint = owner
        .get("endpoint")
        .context("current endpoint identity absent")?;
    let path = PathBuf::from(
        endpoint
            .get("log")
            .and_then(Value::as_str)
            .context("current endpoint log absent")?,
    );
    ensure!(path.parent() == Some(root), "drain log escapes owned root");
    let bytes = artifacts::read(&path)?;
    ensure!(
        certificate.get("log_sha256").and_then(Value::as_str)
            == Some(artifacts::sha(&bytes).as_str()),
        "current endpoint log differs from certified evidence"
    );
    let counts = parse(std::str::from_utf8(&bytes)?)?;
    ensure!(
        certificate.get("counts") == Some(&serde_json::to_value(counts)?),
        "current drain accounting differs from actual log"
    );
    Ok(certificate)
}

pub fn verify_identity(owner: &Value, certificate: &Value, boot: &str) -> Result<()> {
    ensure!(
        owner.get("boot_id").and_then(Value::as_str) == Some(boot),
        "drain belongs to another boot"
    );
    ensure!(
        certificate.get("identity") == Some(owner),
        "drain belongs to an old supervisor or endpoint incarnation"
    );
    ensure!(
        certificate.get("orderly").and_then(Value::as_bool) == Some(true),
        "current supervisor drain failed"
    );
    let endpoint = certificate
        .get("endpoint_exit")
        .context("current endpoint reap evidence absent")?;
    ensure!(
        endpoint.get("identity") == owner.get("endpoint")
            && endpoint.get("reaped").and_then(Value::as_bool) == Some(true),
        "endpoint drain/reap identity differs"
    );
    ensure!(
        endpoint
            .get("exit")
            .and_then(|exit| exit.get("code"))
            .and_then(Value::as_i64)
            == Some(0),
        "endpoint did not exit successfully after producing current drain"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
