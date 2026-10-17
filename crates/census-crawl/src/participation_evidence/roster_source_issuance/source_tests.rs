use super::*;
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use census_domain::model::{SchoolId, SourceNamespace, SourceSubject, TeamId};
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_claim, RosterAuthorityLimitsV1,
};
use serde_json::{json, Value};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const DIRECTORY: &str = include_str!("../provider_school/fixtures/mi-team-index.html");
const ROSTER: &str = include_str!("../provider_school/fixtures/dcc-provider-roster.html");
const URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";

fn retained(url: &str, body: &str) -> TestResult<RetainedHtmlCaptureV1> {
    let instant = DateTime::parse_from_rfc3339("2026-10-16T00:49:43Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        instant,
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: Vec::new(),
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: instant,
        },
    )?)
}

fn fixture(body: &str) -> TestResult<ProviderSchoolPacketV1> {
    Ok(ProviderSchoolPacketV1::new(
        retained("https://mi.milesplit.com/teams", DIRECTORY)?,
        retained(URL, body)?,
    )?)
}

fn reference(index: u32) -> Value {
    json!({"digest":format!("{index:064x}"), "bytes":64})
}

fn proposal(packet: &ProviderSchoolPacketV1, body: &str) -> TestResult<Value> {
    let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, "12439".into())?;
    let program = SourceSubject::new(SourceNamespace::MilesplitTeam, "12439".into())?;
    let contract = json!({"version":1, "revision":1, "digest":"a".repeat(64)});
    let encoded = packet.encode()?;
    Ok(json!({
        "version":1,
        "scope":{"provider":"milesplit", "owner":owner, "sport":"cross_country", "category":"boys",
            "policy":contract, "relationship":"represents_school_program", "temporal":"exact_utc_capture_day"},
        "parser":{"artifact":reference(1), "interpretation":CurrentRosterAssertion::INTERPRETATION_REVISION,
            "schema":reference(2), "configuration":reference(3), "dependencies":reference(4)},
        "capture":{"occurrence":"b".repeat(64), "body":{"digest":hash(body.as_bytes()), "bytes":body.len()},
            "manifest":reference(6), "interpretation":reference(7), "acquired_at":"2026-10-16T00:49:43Z",
            "observed_day":"2026-10-16", "requested_url":URL, "final_url":{"status":"observed", "url":URL},
            "retained_source_admission":reference(8), "verification_grant":reference(9)},
        "binding":{"school":SchoolId::mint("sch", &["source-assessment"]),
            "program":TeamId::mint("team", &["source-assessment"]), "school_source":owner,
            "program_source":program, "sport":"cross_country", "category":"boys", "school_year":2026,
            "year_convention":contract, "projection_merge_contract":contract, "school_projection":reference(10), "program_projection":reference(11),
            "provider_packet":{"digest":hash(&encoded), "bytes":encoded.len()},
            "physical_projection_inventory":reference(13), "store":"c".repeat(64), "snapshot_revision":1},
        "semantic_review_evidence":reference(14)
    }))
}

fn decode(value: &Value) -> TestResult<LocallyCheckedRosterIssuanceV1> {
    Ok(decode_roster_issuance_claim(
        &serde_json::to_vec(value)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?,
    )?)
}

#[test]
fn source_capture_mismatch_and_unknown_final_url_refuse() -> TestResult {
    let packet = fixture(ROSTER)?;
    for (path, replacement) in [
        ("/capture/body/digest", json!("d".repeat(64))),
        ("/capture/final_url", json!({"status":"unknown"})),
        (
            "/capture/requested_url",
            json!("https://mi.milesplit.com/teams/999-other/roster"),
        ),
        ("/capture/acquired_at", json!("2026-10-16T00:49:44Z")),
    ] {
        let mut value = proposal(&packet, ROSTER)?;
        *value.pointer_mut(path).ok_or("fixture field")? = replacement;
        check!(capture(
            &packet,
            &decode(&value)?,
            &mut AttachmentReplayGrantV1::new()
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn missing_current_statement_refuses_program_assessment() -> TestResult {
    let body = ROSTER.replace("<h4>Current Roster</h4>", "<h4>Past Roster</h4>");
    let packet = fixture(&body)?;
    let proposal = decode(&proposal(&packet, &body)?)?;
    check!(scope(
        &packet.read()?,
        &proposal,
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    Ok(())
}

#[test]
fn utc_equivalent_timestamp_is_same_capture_instant() -> TestResult {
    let packet = fixture(ROSTER)?;
    let mut value = proposal(&packet, ROSTER)?;
    *value
        .pointer_mut("/capture/acquired_at")
        .ok_or("fixture instant")? = json!("2026-10-15T19:49:43-05:00");
    capture(
        &packet,
        &decode(&value)?,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    Ok(())
}

#[test]
fn underdeclared_body_cannot_bypass_real_capture_precharge() -> TestResult {
    let packet = fixture(ROSTER)?;
    let mut value = proposal(&packet, ROSTER)?;
    *value
        .pointer_mut("/capture/body/bytes")
        .ok_or("fixture body size")? = json!(1);
    let proposal = decode(&value)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let maximum = census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes;
    grant.charge_bytes(maximum.checked_sub(1).ok_or("grant limit")?)?;
    let before = grant.usage().encoded_bytes();
    check!(capture(&packet, &proposal, &mut grant).is_err());
    check!(eq; grant.usage().encoded_bytes(), before);
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn canonical_packet_allocation_requires_real_precharge() -> TestResult {
    let packet = fixture(ROSTER)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let maximum = census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes;
    grant.charge_bytes(maximum.checked_sub(1).ok_or("grant limit")?)?;
    check!(budget::charge(&packet, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[path = "source_integration_tests.rs"]
pub(in crate::participation_evidence::roster_source_issuance) mod integration;

#[path = "source_review_integration_tests.rs"]
mod review_integration;

#[path = "source_review_reader_tests.rs"]
mod review_reader;
