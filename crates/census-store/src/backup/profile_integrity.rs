use super::{CorruptionClass, IntegrityCorruption};
use crate::{StoreError, StoreResult};
use fjall::{Keyspace, Snapshot};

mod classify;
mod key;
mod scan;

pub(super) const MAX_RECORDS: u64 = 25_000;
pub(super) const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FINDINGS: usize = 1024;
const MAX_LIMITS: usize = 16;
const MAX_LABEL_BYTES: usize = 128;
const PREFIX: &[u8] = b"profiles:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    Complete,
    Incomplete,
}

#[derive(Debug)]
pub(super) struct Report {
    pub(super) scope: Scope,
    pub(super) records: u64,
    pub(super) bytes: u64,
    pub(super) corruptions: Vec<IntegrityCorruption>,
    pub(super) limits: Vec<String>,
    record_window: u64,
    byte_window: u64,
}

pub(super) fn check(
    snapshot: &Snapshot,
    meta: &Keyspace,
    records: u64,
    bytes: u64,
) -> StoreResult<Report> {
    let mut report = Report::new(records, bytes);
    scan::preflight(snapshot, meta, &mut report)?;
    if report.scope == Scope::Complete {
        scan::decode(snapshot, meta, &mut report)?;
    }
    Ok(report)
}

pub(super) fn require(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<()> {
    let report = check(snapshot, meta, MAX_RECORDS, MAX_BYTES)?;
    if let Some(finding) = report.corruptions.first() {
        return Err(StoreError::Refused {
            detail: format!(
                "profile metadata cannot be accepted: {:?} at {}: {}",
                finding.class, finding.subject, finding.detail
            ),
        });
    }
    if report.scope == Scope::Incomplete {
        let reason = report
            .limits
            .first()
            .map_or("unspecified incomplete scope", String::as_str);
        return Err(StoreError::Refused {
            detail: format!("profile metadata verification is incomplete: {reason}"),
        });
    }
    Ok(())
}

impl Report {
    fn new(records: u64, bytes: u64) -> Self {
        Self {
            scope: Scope::Complete,
            records: 0,
            bytes: 0,
            corruptions: Vec::new(),
            limits: Vec::new(),
            record_window: records.min(MAX_RECORDS),
            byte_window: bytes.min(MAX_BYTES),
        }
    }

    fn admit(&mut self, key_bytes: usize, value_bytes: u32) -> StoreResult<bool> {
        let key_bytes = u64::try_from(key_bytes).map_err(|_| StoreError::CounterOverflow)?;
        let bytes = key_bytes
            .checked_add(u64::from(value_bytes))
            .ok_or(StoreError::CounterOverflow)?;
        let records = self
            .records
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let next = self
            .bytes
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        if records > self.record_window || next > self.byte_window {
            self.incomplete(format!(
                "profile integrity stopped at the ADR-036 metadata window ({} records or {} key-plus-value bytes); retained content past it is not verified",
                self.record_window, self.byte_window
            ));
            return Ok(false);
        }
        self.records = records;
        self.bytes = next;
        Ok(true)
    }

    fn incomplete(&mut self, detail: String) {
        self.scope = Scope::Incomplete;
        if self.limits.len() < MAX_LIMITS {
            self.limits.push(detail);
        }
    }

    fn note(&mut self, class: CorruptionClass, key: &[u8], detail: String) {
        if self.corruptions.len() == MAX_FINDINGS {
            self.incomplete(format!(
                "profile integrity stopped after {MAX_FINDINGS} findings"
            ));
            return;
        }
        self.corruptions.push(IntegrityCorruption {
            class,
            subject: label(key),
            detail,
        });
    }
}

fn label(key: &[u8]) -> String {
    let prefix = key
        .get(..key.len().min(MAX_LABEL_BYTES))
        .map_or(&[][..], core::convert::identity);
    format!(
        "meta/{} [key_bytes={}]",
        crate::keys::key_label(prefix),
        key.len()
    )
}

#[cfg(test)]
mod tests;
