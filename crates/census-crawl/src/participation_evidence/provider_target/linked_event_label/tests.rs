use super::super::super::{
    hash, RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use super::super::{CompetitionCategoryEvidenceV1, SourceTargetPacketV1};
use census_domain::model::SourceNamespace;
use chrono::{DateTime, Utc};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const PROFILE: &[u8] = include_bytes!("../tests/fixtures/profile.html");
const RESULTS: &[u8] = include_bytes!("../tests/fixtures/linked-results.html");
const PROFILE_URL: &str = "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks";
const RESULTS_URL: &str =
    "https://mi.milesplit.com/meets/760382-chsl-championships-2026/teams/12439";

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
            include_bytes!("../tests/fixtures/profile.headers"),
            "2026-10-16T00:50:21Z",
            "2026-10-16T00:50:27Z",
        )?,
        capture(
            RESULTS_URL,
            results,
            include_bytes!("../tests/fixtures/linked-results.headers"),
            "2026-10-16T00:49:13Z",
            "2026-10-16T00:49:19Z",
        )?,
    )?)
}

#[test]
fn genuine_brocks_110m_hurdles_row_produces_checked_label_with_source_backed_proof() -> TestResult {
    let packet = packet(PROFILE, RESULTS)?;
    let intake = packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed { rows, .. } = intake.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    let hurdles_row = rows
        .iter()
        .find(|row| row.event_code() == "10")
        .ok_or("110m hurdles row absent")?;

    let checked = packet.read_event_label(hurdles_row.proof())?;

    check!(eq;
        checked.label(),
        "Boys 110 Meter Hurdles"
    );
    check!(eq;
        checked.source_url(),
        RESULTS_URL
    );
    check!(eq;
        checked.person().namespace,
        SourceNamespace::MilesplitAthlete
    );
    check!(eq; checked.person().id, "7223777");
    check!(eq; checked.row_proof().locator(), hurdles_row.proof().locator());
    check!(checked.heading_proof().locator().starts_with("html_bytes:"));
    check!(checked.table_proof().locator().starts_with("html_bytes:"));
    check!(checked.heading_proof().locator() != checked.table_proof().locator());

    Ok(())
}

#[test]
fn genuine_brocks_300m_hurdles_row_produces_checked_label() -> TestResult {
    let packet = packet(PROFILE, RESULTS)?;
    let intake = packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed { rows, .. } = intake.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    let hurdles_row = rows
        .iter()
        .find(|row| row.event_code() == "42")
        .ok_or("300m hurdles row absent")?;

    let checked = packet.read_event_label(hurdles_row.proof())?;

    check!(eq;
        checked.label(),
        "Boys 300 Meter Hurdles"
    );
    check!(eq;
        checked.source_url(),
        RESULTS_URL
    );
    check!(eq; checked.row_proof().locator(), hurdles_row.proof().locator());

    Ok(())
}

#[test]
fn row_proof_must_match_exact_row_not_another_row() -> TestResult {
    let packet = packet(PROFILE, RESULTS)?;
    let intake = packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed { rows, .. } = intake.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    let hurdles_row = rows
        .iter()
        .find(|row| row.event_code() == "10")
        .ok_or("110m hurdles row absent")?;
    let three_hundred_row = rows
        .iter()
        .find(|row| row.event_code() == "42")
        .ok_or("300m hurdles row absent")?;

    let hurdles_checked = packet.read_event_label(hurdles_row.proof())?;
    check!(eq;
        hurdles_checked.label(),
        "Boys 110 Meter Hurdles"
    );
    check!(eq;
        hurdles_checked.row_proof().locator(),
        hurdles_row.proof().locator()
    );

    let three_hundred_checked = packet.read_event_label(three_hundred_row.proof())?;
    check!(eq;
        three_hundred_checked.label(),
        "Boys 300 Meter Hurdles"
    );
    check!(eq;
        three_hundred_checked.row_proof().locator(),
        three_hundred_row.proof().locator()
    );

    check!(hurdles_checked.row_proof().locator() != three_hundred_checked.row_proof().locator());
    Ok(())
}

#[test]
fn synthetic_aria_labelledby_pointing_to_missing_id_fails() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let changed = html.replace(
        "aria-labelledby=\"teamResultGroup6\"",
        "aria-labelledby=\"missing-id\"",
    );
    let packet = packet(PROFILE, changed.as_bytes())?;
    let intake = packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed { rows, .. } = intake.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    let hurdles_row = rows
        .iter()
        .find(|row| row.event_code() == "10")
        .ok_or("110m hurdles row absent")?;
    check!(packet.read_event_label(hurdles_row.proof()).is_err());
    Ok(())
}

#[test]
fn synthetic_duplicate_aria_labelledby_ids_fails() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let changed = html
        .replace(
            "<h3 id=\"teamResultGroup6\" class=\"meetTeams__groupTitle\">",
            "<h3 id=\"teamResultGroup7\" class=\"meetTeams__groupTitle\">",
        )
        .replace(
            "aria-labelledby=\"teamResultGroup6\"",
            "aria-labelledby=\"teamResultGroup7\"",
        );
    let packet = packet(PROFILE, changed.as_bytes())?;
    let intake = packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed { rows, .. } = intake.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    let hurdles_row = rows
        .iter()
        .find(|row| row.event_code() == "10")
        .ok_or("110m hurdles row absent")?;
    check!(packet.read_event_label(hurdles_row.proof()).is_err());
    Ok(())
}

#[test]
fn synthetic_hidden_heading_fails() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let changed = html.replace(
        "<h3 id=\"teamResultGroup6\" class=\"meetTeams__groupTitle\">",
        "<h3 id=\"teamResultGroup6\" class=\"meetTeams__groupTitle\" hidden>",
    );
    let packet = packet(PROFILE, changed.as_bytes())?;
    let intake = packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed { rows, .. } = intake.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    let hurdles_row = rows
        .iter()
        .find(|row| row.event_code() == "10")
        .ok_or("110m hurdles row absent")?;
    check!(packet.read_event_label(hurdles_row.proof()).is_err());
    Ok(())
}

#[test]
fn synthetic_table_missing_aria_labelledby_fails() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let changed = html.replace("aria-labelledby=\"teamResultGroup6\"", "");
    let packet = packet(PROFILE, changed.as_bytes())?;
    let intake = packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed { rows, .. } = intake.categories() else {
        return Err("genuine linked category was not agreed".into());
    };
    let hurdles_row = rows
        .iter()
        .find(|row| row.event_code() == "10")
        .ok_or("110m hurdles row absent")?;
    check!(packet.read_event_label(hurdles_row.proof()).is_err());
    Ok(())
}
#[test]
fn synthetic_cross_person_proof_refused_same_shared_body() -> TestResult {
    let profile_str = std::str::from_utf8(PROFILE)?;
    let owen_profile = profile_str
        .replace("7223777", "14570824")
        .replace("zacchaeus-brocks", "owen-courser")
        .replace("Zacchaeus Brocks", "Owen Courser")
        .replace("Zacchaeus", "Owen")
        .replace("Gender\": \"Female\"", "Gender\": \"Male\"");
    let owen_url = "https://mi.milesplit.com/athletes/14570824-owen-courser";

    let brocks_packet = packet(PROFILE, RESULTS)?;
    let owen_packet = SourceTargetPacketV1::new(
        capture(
            owen_url,
            owen_profile.as_bytes(),
            include_bytes!("../tests/fixtures/profile.headers"),
            "2026-10-16T00:50:21Z",
            "2026-10-16T00:50:27Z",
        )?,
        capture(
            RESULTS_URL,
            RESULTS,
            include_bytes!("../tests/fixtures/linked-results.headers"),
            "2026-10-16T00:49:13Z",
            "2026-10-16T00:49:19Z",
        )?,
    )?;

    check!(
        brocks_packet.linked_results.body_sha256() == owen_packet.linked_results.body_sha256(),
        "two packets must share identical retained results body"
    );

    let brocks_intake = brocks_packet.read()?;
    let owen_intake = owen_packet.read()?;

    check!(eq; brocks_intake.profile().person().id, "7223777");
    check!(eq; owen_intake.profile().person().id, "14570824");
    check!(brocks_intake.profile().person().id != owen_intake.profile().person().id);

    let brocks_hurdles = brocks_intake
        .categories()
        .rows()
        .iter()
        .find(|r| r.event_code() == "10")
        .ok_or("Brock 110m row absent")?;

    let owen_400 = owen_intake
        .categories()
        .rows()
        .iter()
        .find(|r| r.event_code() == "56")
        .ok_or("Owen 400m row absent")?;

    let brocks_checked = brocks_packet.read_event_label(brocks_hurdles.proof())?;
    check!(eq; brocks_checked.label(), "Boys 110 Meter Hurdles");
    check!(eq; brocks_checked.person().id, "7223777");

    let owen_checked = owen_packet.read_event_label(owen_400.proof())?;
    check!(eq; owen_checked.label(), "Boys 400 Meter Dash");
    check!(eq; owen_checked.person().id, "14570824");

    check!(
        brocks_packet.read_event_label(owen_400.proof()).is_err(),
        "Brock packet must refuse Owen's row proof despite same results body"
    );
    check!(
        owen_packet
            .read_event_label(brocks_hurdles.proof())
            .is_err(),
        "Owen packet must refuse Brock's row proof despite same results body"
    );

    Ok(())
}

#[test]
fn synthetic_altered_body_with_old_proof_fails() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let original_row = html.get(65480..66799).ok_or("retained row span absent")?;
    check!(eq;
        hash(original_row.as_bytes()),
        "93fa4b8db2c428b36aacd581f89720d3dc49533e400064803178cb4f05f1453d"
    );

    let altered_row = original_row.replace("13.26", "99.99");
    let changed_html = html.replacen(original_row, &altered_row, 1);
    let changed_packet = packet(PROFILE, changed_html.as_bytes())?;

    let genuine_packet = packet(PROFILE, RESULTS)?;
    let genuine_intake = genuine_packet.read()?;
    let CompetitionCategoryEvidenceV1::Agreed {
        rows: genuine_rows, ..
    } = genuine_intake.categories()
    else {
        return Err("genuine linked category was not agreed".into());
    };
    let genuine_row = genuine_rows
        .iter()
        .find(|row| row.event_code() == "10")
        .ok_or("genuine 110m hurdles row absent")?;

    check!(changed_packet
        .read_event_label(genuine_row.proof())
        .is_err());
    let genuine = genuine_packet.read_event_label(genuine_row.proof())?;
    check!(eq;
        genuine.label(),
        "Boys 110 Meter Hurdles"
    );

    Ok(())
}
