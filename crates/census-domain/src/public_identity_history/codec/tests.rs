use serde::de::DeserializeSeed;

use super::*;
use crate::model::{AthleteCandidateId, SourceNamespace, StoreIdentity};
use crate::public_identity_history::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod boundaries;
mod fixture;
mod replay;
mod wire;

fn decode(
    raw: &[u8],
    fixture: &fixture::Fixture,
    limits: ReplayLimits,
    byte_limit: usize,
) -> TestResult<DecodedCandidateHistory> {
    if raw.len() > byte_limit {
        return Err("shell encoded history byte limit".into());
    }
    let mut deserializer = serde_json::Deserializer::from_slice(raw);
    let decoded = CandidateHistorySeed::new(&fixture.installation, &fixture.head, limits)
        .deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(decoded)
}

fn error(raw: &[u8], fixture: &fixture::Fixture, limits: ReplayLimits) -> TestResult<String> {
    match decode(raw, fixture, limits, 1024 * 1024) {
        Ok(_) => Err("expected history decoding refusal".into()),
        Err(error) => Ok(error.to_string()),
    }
}

fn replace(value: &mut serde_json::Value, pointer: &str, next: serde_json::Value) -> TestResult {
    *value
        .pointer_mut(pointer)
        .ok_or("missing test JSON pointer")? = next;
    Ok(())
}

fn roundtrip(fixture: &fixture::Fixture) -> TestResult {
    let raw = serde_json::to_vec(&fixture.records)?;
    let decoded = decode(&raw, fixture, ReplayLimits::SMALL, 1024 * 1024)?;
    check!(eq; decoded.records(), fixture.records.as_slice());
    check!(eq; serde_json::to_vec(decoded.records())?, raw);
    check!(eq; decoded.projection().head(), &fixture.head);
    let (records, projection) = decoded.into_parts();
    check!(eq; records, fixture.records);
    check!(eq; projection.head(), &fixture.head);
    Ok(())
}
