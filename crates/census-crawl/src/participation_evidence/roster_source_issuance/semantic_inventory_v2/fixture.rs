use super::*;
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use serde_json::Value;

pub(super) const BODY: &str =
    include_str!("../../provider_school/fixtures/dcc-provider-roster.html");
const DIRECTORY: &str = include_str!("../../provider_school/fixtures/mi-team-index.html");
const MANIFEST: &[u8] = include_bytes!("../../provider_school/fixtures/capture-manifest.json");
pub(super) const URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";
pub(super) const AT: &str = "2026-10-16T00:49:43Z";

pub(super) fn original_bytes() -> TestResult {
    check!(eq; BODY.len(), 372_977);
    check!(eq; crate::net::cache::content_digest(BODY.as_bytes()), "246374a87ba6d56802e58e54e4a1edcc5d1eee52106dedb24df185cbd037e178");
    check!(eq; DIRECTORY.len(), 293_847);
    check!(eq; crate::net::cache::content_digest(DIRECTORY.as_bytes()), "39606bf8e07f0170138bb586ae8a762903bb820b515520e89e34745f6c732351");
    Ok(())
}

pub(super) fn packet() -> TestResult<ProviderSchoolPacketV1> {
    original_bytes()?;
    let manifest: Value = serde_json::from_slice(MANIFEST)?;
    Ok(ProviderSchoolPacketV1::new(
        capture(
            &manifest,
            "mi-team-index",
            DIRECTORY,
            include_bytes!("../../provider_school/fixtures/mi-team-index.headers"),
        )?,
        capture(
            &manifest,
            "dcc-provider-roster",
            BODY,
            include_bytes!("../../provider_school/fixtures/dcc-provider-roster.headers"),
        )?,
    )?)
}

pub(super) fn span(row: SourceRowLocator) -> TestResult<&'static str> {
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("row span overflow")?;
    Ok(BODY
        .get(row.byte_offset..end)
        .ok_or("exact original UTF-8 row span")?)
}

fn capture(
    manifest: &Value,
    label: &str,
    body: &str,
    headers: &[u8],
) -> TestResult<RetainedHtmlCaptureV1> {
    let sources = manifest
        .get("captures")
        .and_then(Value::as_array)
        .ok_or("retained capture manifest")?;
    let source = sources
        .iter()
        .find(|value| value.get("label").and_then(Value::as_str) == Some(label))
        .ok_or("retained capture label")?;
    let text = |key| {
        source
            .get(key)
            .and_then(Value::as_str)
            .ok_or("retained source field")
    };
    check!(eq; source.get("status").and_then(Value::as_u64), Some(200));
    Ok(RetainedHtmlCaptureV1::new(
        text("requested_url")?,
        text("final_url")?,
        200,
        chrono::DateTime::parse_from_rfc3339(text("completed_at")?)?.to_utc(),
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: text("request_method")?.into(),
            request_headers: serde_json::from_value(
                source
                    .get("request_headers")
                    .ok_or("request headers")?
                    .clone(),
            )?,
            response_header_bytes: headers.to_vec(),
            content_type: text("content_type")?.into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: chrono::DateTime::parse_from_rfc3339(text("started_at")?)?.to_utc(),
        },
    )?)
}
