use crate::{CrawlError, CrawlResult};
use census_store::{Store, MAX_OPERATION_BYTES};
use serde::Serialize;
use serde_json::Value;

pub(super) const PHASE: &str = "crawl_effect_receipts_v1";

#[derive(Serialize)]
pub(super) struct Witness<'a> {
    pub(super) digest: &'a str,
}

#[derive(Clone, Copy)]
pub(super) struct Effect<'a> {
    operation: &'a str,
    digest: &'a str,
}

impl<'a> Effect<'a> {
    pub(super) fn new(operation: &'a str, digest: &'a str) -> CrawlResult<Self> {
        validate(operation, digest)?;
        Ok(Self { operation, digest })
    }

    pub(super) fn operation(self) -> &'a str {
        self.operation
    }
    pub(super) fn digest(self) -> &'a str {
        self.digest
    }
    pub(super) fn witness(self) -> Witness<'a> {
        Witness {
            digest: self.digest,
        }
    }
    pub(super) fn seen(self, payload: Option<&Value>) -> CrawlResult<bool> {
        seen(payload, self.digest)
    }
    pub(super) fn committed(self, store: &Store) -> CrawlResult<bool> {
        committed(store, self.operation, self.digest)
    }
}

fn validate(operation: &str, digest: &str) -> CrawlResult<()> {
    if operation.is_empty() || operation.len() > MAX_OPERATION_BYTES {
        return Err(CrawlError::Resource {
            resource: "source effect operation bytes",
            requested: operation.len(),
            limit: MAX_OPERATION_BYTES,
        });
    }
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invariant(
            "source effect digest must be a SHA-256 hexadecimal value",
        ));
    }
    Ok(())
}

fn seen(payload: Option<&Value>, digest: &str) -> CrawlResult<bool> {
    let Some(payload) = payload else {
        return Ok(false);
    };
    let actual = payload
        .get("digest")
        .and_then(Value::as_str)
        .ok_or_else(|| invariant("source effect witness has no digest"))?;
    compare(actual, digest)?;
    Ok(true)
}

fn committed(store: &Store, operation: &str, digest: &str) -> CrawlResult<bool> {
    let payload = store.journal_payload(PHASE, operation)?;
    if seen(payload.as_ref(), digest)? {
        return Ok(true);
    }
    let Some(receipt) = store.receipt(operation)? else {
        return Ok(false);
    };
    compare(&receipt.digest, digest)?;
    Ok(true)
}

fn compare(actual: &str, expected: &str) -> CrawlResult<()> {
    if actual != expected {
        return Err(invariant(
            "one source effect operation cannot name two payloads",
        ));
    }
    Ok(())
}

fn invariant(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.to_owned(),
    }
}
