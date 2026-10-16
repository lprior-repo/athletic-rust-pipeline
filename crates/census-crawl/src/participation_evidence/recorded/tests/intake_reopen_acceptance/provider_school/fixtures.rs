use super::*;
use crate::milesplit::{parse_team_index, roster_entities, Site};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use chrono::{DateTime, Utc};

pub(super) const DIRECTORY: &[u8] =
    include_bytes!("../../../../provider_school/fixtures/mi-team-index.html");
pub(super) const ROSTER: &[u8] =
    include_bytes!("../../../../provider_school/fixtures/dcc-provider-roster.html");
const DIRECTORY_URL: &str = "https://mi.milesplit.com/teams";
const ROSTER_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";

fn capture(
    url: &str,
    body: &[u8],
    headers: &[u8],
    start: &str,
    end: &str,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339(end)?.with_timezone(&Utc),
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "text/html".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(start)?.with_timezone(&Utc),
        },
    )?)
}

pub(super) fn packet(directory: &[u8]) -> TestResult<ProviderSchoolPacketV1> {
    Ok(ProviderSchoolPacketV1::new(
        capture(
            DIRECTORY_URL,
            directory,
            include_bytes!("../../../../provider_school/fixtures/mi-team-index.headers"),
            "2026-10-16T02:57:54Z",
            "2026-10-16T02:58:00Z",
        )?,
        capture(
            ROSTER_URL,
            ROSTER,
            include_bytes!("../../../../provider_school/fixtures/dcc-provider-roster.headers"),
            "2026-10-16T00:49:38Z",
            "2026-10-16T00:49:43Z",
        )?,
    )?)
}

pub(super) fn verify_checked_school(checked: &CheckedProviderSchoolPacketV1) -> TestResult {
    check!(eq; DIRECTORY.len(), 293_847);
    check!(eq; hash(DIRECTORY), "39606bf8e07f0170138bb586ae8a762903bb820b515520e89e34745f6c732351");
    check!(eq; ROSTER.len(), 372_977);
    check!(eq; hash(ROSTER), "246374a87ba6d56802e58e54e4a1edcc5d1eee52106dedb24df185cbd037e178");
    let parsed = parse_team_index(std::str::from_utf8(DIRECTORY)?)?;
    check!(eq; parsed.disposition, crate::CollectionDisposition::Complete);
    check!(eq; parsed.errors, 0);
    check!(parsed.unfinished.is_empty());
    let mut selected = parsed.teams.iter().filter(|team| team.id == "12439");
    let team = selected.next().ok_or("genuine directory school")?;
    check!(selected.next().is_none());
    let year = SchoolYear::from_date("2026-10-16").ok_or("genuine roster acquisition year")?;
    let (expected, athletes, teams) = roster_entities(
        team,
        &[],
        year,
        "2026-10-16T00:49:43+00:00",
        &Site::for_jurisdiction(UsJurisdiction::MI),
    )?;
    check!(athletes.is_empty());
    check!(teams.is_empty());
    let school = checked.school();
    check!(eq; school.id, expected.id);
    check!(eq; school.source_identities, expected.source_identities);
    let [identity] = school.source_identities.as_slice() else {
        return Err("exactly one genuine provider school owner required".into());
    };
    check!(eq; identity.namespace, SourceNamespace::MilesplitSchool);
    check!(eq; identity.id, "12439");
    check!(eq; identity.url.as_deref(), Some("https://mi.milesplit.com/teams/12439-detroit-catholic-central"));
    check!(eq; school.name, "Detroit Catholic Central");
    check!(eq; school.city.as_deref(), Some("Novi"));
    check!(eq; school.state, Some(UsJurisdiction::MI));
    check!(school.postal_addresses.is_empty());
    let proof = checked.directory_proof();
    check!(eq; proof.capture_sha256(), hash(DIRECTORY));
    check!(eq; proof.acquired_at().to_rfc3339(), "2026-10-16T02:58:00+00:00");
    check!(eq; proof.disposition(), crate::CollectionDisposition::Complete);
    check!(eq; proof.parsed_teams(), parsed.teams.len());
    check!(eq; proof.errors(), 0);
    check!(proof.unfinished().is_empty());
    check!(eq; proof.row(), SourceRowLocator { ordinal: 241, byte_offset: 91718, byte_length: 271 });
    check!(eq; proof.name(), SourceRowLocator { ordinal: 241, byte_offset: 91735, byte_length: 176 });
    check!(eq; proof.city(), SourceRowLocator { ordinal: 241, byte_offset: 91924, byte_length: 51 });
    check!(eq; proof.anchor(), SourceRowLocator { ordinal: 241, byte_offset: 91756, byte_length: 137 });
    check!(eq; proof.row_sha256(), "88e0862298243b4906a79b4ba4e1649a1385d646e914016c4186b15447421981");
    let observation = checked.observation();
    check!(eq; observation.namespace, SourceNamespace::MilesplitSchool);
    check!(eq; observation.source_school_id, "12439");
    check!(eq; observation.observed_name, "Detroit Catholic Central");
    check!(eq; observation.city.as_deref(), Some("Novi"));
    check!(eq; observation.observed_on, "2026-10-16T02:58:00+00:00");
    check!(eq; observation.source_row_key,
        format!("provider_school_intake_v1:{}:directory:241:91718:271", checked.reference().as_str()));
    let roster = checked
        .roster_verdict()
        .roster()
        .ok_or("genuine parsed roster")?;
    check!(eq; &roster.team, team);
    check!(roster
        .athletes
        .iter()
        .all(|row| row.athlete().athlete_id != "7223777"));
    check!(!std::str::from_utf8(ROSTER)?.contains("Brocks"));
    check!(eq; school.evidence.len(), 2);
    for (evidence, (role, at, digest)) in school.evidence.iter().zip([
        (
            "directory_name_city",
            "2026-10-16T02:58:00+00:00",
            hash(DIRECTORY),
        ),
        (
            "provider_roster_owner",
            "2026-10-16T00:49:43+00:00",
            hash(ROSTER),
        ),
    ]) {
        check!(eq; evidence.observed_on, at);
        let note: Value = serde_json::from_str(
            evidence
                .note
                .as_deref()
                .ok_or("source-backed school provenance")?,
        )?;
        check!(eq; note.get("role").and_then(Value::as_str), Some(role));
        check!(eq; note.get("packet").and_then(Value::as_str), Some(checked.reference().as_str()));
        check!(eq; note.get("capture_sha256").and_then(Value::as_str), Some(digest.as_str()));
    }
    Ok(())
}
