use super::super::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use super::*;
use census_domain::model::{Gender, SourceNamespace};
use chrono::{DateTime, Utc};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const PROFILE: &[u8] = include_bytes!("tests/fixtures/profile.html");
const RESULTS: &[u8] = include_bytes!("tests/fixtures/linked-results.html");
const PROFILE_URL: &str = "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks";
const RESULTS_URL: &str =
    "https://mi.milesplit.com/meets/760382-chsl-championships-2026/teams/12439";

mod boundaries;

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

fn packet(profile: &[u8], results: &[u8]) -> TestResult<SourceTargetPacketV1> {
    Ok(SourceTargetPacketV1::new(
        capture(
            PROFILE_URL,
            profile,
            include_bytes!("tests/fixtures/profile.headers"),
            "2026-10-16T00:50:21Z",
            "2026-10-16T00:50:27Z",
        )?,
        capture(
            RESULTS_URL,
            results,
            include_bytes!("tests/fixtures/linked-results.headers"),
            "2026-10-16T00:49:13Z",
            "2026-10-16T00:49:19Z",
        )?,
    )?)
}

#[test]
fn genuine_profile_and_linked_rows_preserve_distinct_source_scopes() -> TestResult {
    check!(eq;
        hash(PROFILE),
        "b11d89fd95b7e08e5a90d1cbc1e51174cdb4a5f1bdf3833b980f4256ae1b266c"
    );
    check!(eq;
        hash(RESULTS),
        "3a8c7b8c30d42117c784c8401acebd07d4cd804ea58079d244d29682b979393d"
    );
    let value = packet(PROFILE, RESULTS)?.read()?;
    let profile = value.profile();
    check!(eq;
        profile.person().namespace,
        SourceNamespace::MilesplitAthlete
    );
    check!(eq; profile.person().id, "7223777");
    check!(eq; profile.school().namespace, SourceNamespace::MilesplitSchool);
    check!(eq; profile.school().id, "12439");
    check!(eq; profile.school_name(), "Detroit Catholic Central");
    check!(eq; profile.name(), "Zacchaeus Brocks");
    check!(eq; profile.cohort().get(), 2026);
    check!(eq; profile.locality(), "Novi, MI");
    check!(eq; profile.person_gender().raw(), Some("Female"));
    check!(eq;
        profile.person_gender().source_scope(),
        "schema_org_person_gender"
    );
    check!(eq; profile.competition_category(), Gender::Unknown);
    let CompetitionCategoryEvidenceV1::Agreed { category, rows } = value.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    check!(eq; *category, Gender::Boys);
    check!(eq; rows.len(), 2);
    check!(eq;
        profile.person_gender().proof().locator(),
        "html_bytes:3359:3621"
    );
    check!(eq;
        rows.iter()
            .map(|row| row.proof().locator())
            .collect::<Vec<_>>(),
        vec!["html_bytes:65480:66799", "html_bytes:69033:70352"]
    );
    check!(eq;
        rows.iter()
            .map(|row| (row.event_code(), row.mark()))
            .collect::<Vec<_>>(),
        vec![("10", "13.26"), ("42", "36.75")]
    );
    for row in rows {
        check!(eq; row.raw(), Some("m"));
        check!(eq; row.meet_id(), "760382");
        check!(eq; row.result_document_id(), "1300204");
        check!(eq; row.meet_day().as_str(), "2026-05-09");
        check!(eq; row.meet_name(), "CHSL Championships");
        check!(eq; row.proof().capture(), hash(RESULTS));
        check!(row.proof().locator().starts_with("html_bytes:"));
    }
    Ok(())
}

#[test]
fn canonical_packet_replay_rechecks_original_sources_and_transport() -> TestResult {
    let original = packet(PROFILE, RESULTS)?;
    let encoded = original.encode()?;
    let replayed = SourceTargetPacketV1::decode(&encoded)?;
    check!(eq; replayed.encode()?, encoded);
    check!(eq; replayed.read()?.reference(), original.read()?.reference());
    check!(eq;
        replayed.read()?.profile().person_gender().raw(),
        Some("Female")
    );
    let mut corrupted: serde_json::Value = serde_json::from_slice(&encoded)?;
    let profile = corrupted
        .get_mut("profile")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("missing profile fixture")?;
    profile.insert(
        "body_sha256".into(),
        serde_json::Value::String("0".repeat(64)),
    );
    let corrupted: SourceTargetPacketV1 = serde_json::from_value(corrupted)?;
    check!(corrupted.read().is_err());
    let mut changed: serde_json::Value = serde_json::from_slice(&encoded)?;
    let provenance = changed
        .get_mut("profile")
        .and_then(|profile| profile.get_mut("provenance"))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("missing provenance fixture")?;
    provenance.insert(
        "content_type".into(),
        serde_json::Value::String("application/json".into()),
    );
    let changed: SourceTargetPacketV1 = serde_json::from_value(changed)?;
    check!(changed.read().is_err());
    Ok(())
}

#[test]
fn synthetic_category_mutations_remain_unavailable_or_conflicted_with_all_rows() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let prefix = "data-search=\"Zacchaeus Brocks\" data-gender=\"m\"";
    let unknown = html.replacen(
        prefix,
        "data-search=\"Zacchaeus Brocks\" data-gender=\"unknown\"",
        1,
    );
    let value = packet(PROFILE, unknown.as_bytes())?.read()?;
    check!(
        matches!(value.categories(), CompetitionCategoryEvidenceV1::Unavailable { rows } if rows.len() == 2)
    );
    let conflict = html.replacen(
        prefix,
        "data-search=\"Zacchaeus Brocks\" data-gender=\"f\"",
        1,
    );
    let value = packet(PROFILE, conflict.as_bytes())?.read()?;
    check!(
        matches!(value.categories(), CompetitionCategoryEvidenceV1::Conflicted { rows } if rows.len() == 2)
    );
    check!(eq; value.profile().person_gender().raw(), Some("Female"));
    let missing = html.replace(prefix, "data-search=\"Zacchaeus Brocks\"");
    check!(
        matches!(packet(PROFILE, missing.as_bytes())?.read()?.categories(),
        CompetitionCategoryEvidenceV1::Unavailable { rows } if rows.len() == 2)
    );
    Ok(())
}
