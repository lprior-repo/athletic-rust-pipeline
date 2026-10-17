use super::*;
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::roster_source_issuance::manifest::compiled_roster_parser_manifest;
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use census_domain::model::GradYear;

const DIRECTORY: &str = include_str!("../../../provider_school/fixtures/mi-team-index.html");
const ROSTER: &str = include_str!("../../../provider_school/fixtures/dcc-provider-roster.html");
const MANIFEST: &[u8] = include_bytes!("../../../provider_school/fixtures/capture-manifest.json");

#[test]
fn genuine_dcc_partial146_inventory_cannot_be_truncated_to_co2027_complete45() -> TestResult {
    check!(eq; ROSTER.len(), 372_977);
    check!(eq; hash(ROSTER.as_bytes()), "246374a87ba6d56802e58e54e4a1edcc5d1eee52106dedb24df185cbd037e178");
    let manifest: Value = serde_json::from_slice(MANIFEST)?;
    let packet = ProviderSchoolPacketV1::new(
        retained(
            &manifest,
            "mi-team-index",
            DIRECTORY,
            include_bytes!("../../../provider_school/fixtures/mi-team-index.headers"),
        )?,
        retained(
            &manifest,
            "dcc-provider-roster",
            ROSTER,
            include_bytes!("../../../provider_school/fixtures/dcc-provider-roster.headers"),
        )?,
    )?;
    let checked = packet.read()?;
    let parser = compiled_roster_parser_manifest()?;
    let crate::milesplit::RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = checked.roster_verdict()
    else {
        return Err("genuine DCC partial verdict must remain visible".into());
    };
    check!(eq; roster.athletes.len(), 146);
    check!(eq; rejected.len(), 18);
    check!(unfinished.is_none());
    check!(eq; roster.athletes.iter().filter(|member| member.athlete().grad_year == GradYear::CO2027).count(), 45);
    let result = capture::inventory(
        &checked,
        parser.canonical_bytes(),
        b"capacity diagnostic only",
        &mut AttachmentReplayGrantV1::new(),
    );
    match result {
        Err(census_store::StoreError::Refused { detail }) => {
            check!(eq; detail, "semantic inventory requires complete checked roster")
        }
        _ => {
            return Err(
                "partial genuine roster must not become a complete semantic inventory".into(),
            )
        }
    }
    Ok(())
}

#[test]
fn controlled_semantic_inventory_encoding_refuses_exact_capacity_overflow() -> TestResult {
    let mut inventory = fixture::inventory(true)?;
    inventory
        .members
        .first_mut()
        .ok_or("controlled member")?
        .name = "x".repeat(LIMIT);
    let size = serialized_bytes(&inventory)?.len();
    check!(size > LIMIT);
    match encoding::encode(&inventory, &mut AttachmentReplayGrantV1::new()) {
        Err(census_store::StoreError::CanonicalJson(
            census_domain::model::CanonicalJsonError::Write(error),
        )) => {
            check!(eq; error.kind(), std::io::ErrorKind::Other);
            check!(eq; error.to_string(), "semantic size limit");
        }
        _ => return Err("expected exact semantic inventory capacity refusal".into()),
    }
    Ok(())
}

fn retained(
    manifest: &Value,
    label: &str,
    body: &str,
    headers: &[u8],
) -> TestResult<RetainedHtmlCaptureV1> {
    let captures = manifest
        .get("captures")
        .and_then(Value::as_array)
        .ok_or("capture manifest")?;
    let capture = captures
        .iter()
        .find(|value| value.get("label").and_then(Value::as_str) == Some(label))
        .ok_or("capture label")?;
    let text = |key| {
        capture
            .get(key)
            .and_then(Value::as_str)
            .ok_or("capture text")
    };
    let acquired = chrono::DateTime::parse_from_rfc3339(text("completed_at")?)?.to_utc();
    Ok(RetainedHtmlCaptureV1::new(
        text("requested_url")?,
        text("final_url")?,
        200,
        acquired,
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: text("request_method")?.into(),
            request_headers: serde_json::from_value(
                capture
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
