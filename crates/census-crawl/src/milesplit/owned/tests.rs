use super::*;
use census_domain::model::{CentiMetres, CentiSeconds, EventKind, Gender, GradYear, Mark};
use serde_json::{json, Value};

pub(super) const TROY: &[u8] = include_bytes!("fixtures/troy_725218.json");

fn page(body: &[u8]) -> OwnedMeetPage {
    match parse_owned_meet(body, 725218) {
        OwnedMeetVerdict::Parsed(page) => page,
        other => panic!("expected parsed rows, got {other:?}"),
    }
}

fn document() -> Value {
    serde_json::from_slice(TROY).expect("projected public source rows")
}

#[test]
fn provider_result_ids_own_distinct_adelyn_field_performances() {
    let page = page(TROY);
    assert_eq!(page.published_rows, 3);
    assert_eq!(page.rejected, Vec::new());
    let long = &page.rows[0];
    let triple = &page.rows[1];
    assert_eq!((long.result_id, triple.result_id), (201782263, 201782277));
    assert_eq!(long.source_athlete, triple.source_athlete);
    assert_eq!(long.source_athlete.id, "14222592");
    assert_eq!(
        long.source_athlete.url.as_deref(),
        Some("https://www.milesplit.com/athletes/14222592-adelyn-spann")
    );
    assert_eq!(
        (long.meet_id, long.result_set_id, long.team_id),
        (725218, 1266814, 38332)
    );
    assert_eq!(long.gender, Gender::Girls);
    assert_eq!(long.grad_year, GradYear::new(2027));
    assert_eq!(long.cohort, OwnedCohort::Published);
    assert_eq!(
        (long.locator.as_str(), triple.locator.as_str()),
        ("data[0]", "data[1]")
    );
    assert_eq!(long.event_kind, EventKind::LongJump);
    assert_eq!(
        long.mark,
        Mark::FieldImperial {
            feet_mark: "13-9".into(),
            metres: CentiMetres::new(419)
        }
    );
    assert_eq!(triple.event_kind, EventKind::TripleJump);
    assert_eq!(
        triple.mark,
        Mark::FieldImperial {
            feet_mark: "30-7".into(),
            metres: CentiMetres::new(932)
        }
    );
    assert_eq!(long.provider["place"], "11");
    assert_eq!(triple.provider["place"], "5");
    assert_eq!(long.provider["heat"], "1");
    assert_eq!(long.provider["round"], "f");
    assert_eq!(long.provider["units"], "165000");
    assert_eq!(triple.provider["units"], "367000");
    assert_eq!(page.completeness, OwnedCompleteness::Unknown);
    assert!(!page.ownership_complete());
}

#[test]
fn captured_provider_time_uses_the_existing_mark_conversion() {
    let page = page(TROY);
    let row = &page.rows[2];
    assert_eq!(row.result_id, 201782806);
    assert_eq!(row.source_athlete.id, "11357806");
    assert_eq!(row.event_kind, EventKind::Track100m);
    assert_eq!(row.mark, Mark::TimeSeconds(CentiSeconds::new(1240)));
    assert_eq!(row.timing, None);
    assert_eq!(row.provider["eventCode"], "100m");
}

#[test]
fn foreign_meet_and_conflicting_profile_are_located_without_dropping_other_rows() {
    for (field, replacement, expected) in [
        ("meetId", json!("745082"), OwnedRejectionKind::ForeignMeet),
        (
            "profileUrl",
            json!("https://www.milesplit.com/athletes/9999-other"),
            OwnedRejectionKind::ProfileMismatch,
        ),
        (
            "profileUrl",
            json!("https://evil.milesplit.com.example/athletes/14222592-other"),
            OwnedRejectionKind::ProfileMismatch,
        ),
        ("athleteId", json!("0"), OwnedRejectionKind::InvalidIdentity),
        (
            "athleteId",
            json!("-1"),
            OwnedRejectionKind::InvalidIdentity,
        ),
        (
            "athleteId",
            json!("014222592"),
            OwnedRejectionKind::InvalidIdentity,
        ),
        ("athleteId", Value::Null, OwnedRejectionKind::MissingOwner),
        ("id", json!(0), OwnedRejectionKind::InvalidIdentity),
        (
            "meetResultsId",
            json!("0"),
            OwnedRejectionKind::InvalidIdentity,
        ),
        ("mark", json!("13-99"), OwnedRejectionKind::InvalidContext),
        ("place", json!("65536"), OwnedRejectionKind::InvalidContext),
        (
            "windReading",
            json!("NaN"),
            OwnedRejectionKind::InvalidContext,
        ),
        (
            "roundName",
            json!({"name":"Finals"}),
            OwnedRejectionKind::InvalidContext,
        ),
        (
            "firstName",
            json!("\nAdelyn"),
            OwnedRejectionKind::InvalidContext,
        ),
        (
            "lastName",
            json!("Spann\u{0000}"),
            OwnedRejectionKind::InvalidContext,
        ),
    ] {
        let mut document = document();
        document["data"][0][field] = replacement;
        let page = page(&serde_json::to_vec(&document).expect("changed source row"));
        assert_eq!(
            page.rows
                .iter()
                .map(|row| row.result_id)
                .collect::<Vec<_>>(),
            vec![201782277, 201782806]
        );
        assert_eq!(page.rejected.len(), 1);
        assert_eq!(page.rejected[0].locator, "data[0]");
        assert_eq!(page.rejected[0].kind, expected, "{field}");
        assert!(!page.ownership_complete());
    }
}

#[test]
fn malformed_and_duplicate_rows_keep_their_capture_locators() {
    let mut document = document();
    document["data"][0] = json!({"id": true});
    let page = page(&serde_json::to_vec(&document).expect("source JSON"));
    assert_eq!(page.rejected[0].kind, OwnedRejectionKind::MalformedRow);
    assert_eq!(page.rejected[0].locator, "data[0]");
    document = self::document();
    document["data"][1]["id"] = json!(201782263);
    let page = self::page(&serde_json::to_vec(&document).expect("source JSON"));
    assert_eq!(page.rejected[0].kind, OwnedRejectionKind::DuplicateResult);
    assert_eq!(page.rejected[0].locator, "data[1]");
    assert_eq!(page.rows[0].result_id, 201782263);
}

#[test]
fn missing_or_invalid_cohort_does_not_manufacture_a_graduation_year() {
    for (token, cohort) in [
        (json!("0"), OwnedCohort::Missing),
        (json!("9"), OwnedCohort::Invalid),
        (Value::Null, OwnedCohort::Missing),
    ] {
        let mut document = document();
        document["data"][0]["gradYear"] = token;
        let page = page(&serde_json::to_vec(&document).expect("source JSON"));
        assert_eq!(page.rows[0].source_athlete.id, "14222592");
        assert_eq!(page.rows[0].grad_year, None);
        assert_eq!(page.rows[0].cohort, cohort);
    }
}

#[test]
fn malformed_document_and_foreign_empty_envelope_are_not_parsed_success() {
    for body in [
        b"<html>Forbidden</html>".as_slice(),
        b"{\"data\":null}",
        b"{\"_embedded\":{\"meet\":{\"id\":\"745082\"}},\"data\":[]}",
    ] {
        assert!(matches!(
            parse_owned_meet(body, 725218),
            OwnedMeetVerdict::Malformed { .. }
        ));
    }
    let empty = page(b"{\"_meta\":{\"status_code\":200},\"data\":[]}");
    assert_eq!(empty.rows, Vec::new());
    assert_eq!(empty.published_rows, 0);
    assert_eq!(empty.completeness, OwnedCompleteness::Unknown);
    assert!(!empty.ownership_complete());
}

#[test]
fn excessive_body_and_row_counts_are_explicit_document_refusals() {
    let oversized = vec![b' '; MAX_OWNED_BODY_BYTES + 1];
    assert_eq!(
        parse_owned_meet(&oversized, 725218),
        OwnedMeetVerdict::Malformed {
            detail: "invalid meet ID or oversized structured response".into(),
        }
    );
    let rows = serde_json::to_vec(&json!({"data": vec![json!({}); MAX_OWNED_ROWS + 1]}))
        .expect("bounded hostile source");
    assert_eq!(
        parse_owned_meet(&rows, 725218),
        OwnedMeetVerdict::Malformed {
            detail: format!("structured data exceeds {MAX_OWNED_ROWS} rows"),
        }
    );
}

#[test]
fn an_owned_result_without_a_published_person_name_is_rejected_at_its_locator() {
    let mut document = document();
    document["data"][0]["firstName"] = json!(" ");
    document["data"][0]["lastName"] = json!("");
    let page = page(&serde_json::to_vec(&document).expect("anonymous result"));
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row.result_id)
            .collect::<Vec<_>>(),
        vec![201782277, 201782806]
    );
    assert_eq!(
        page.rejected,
        vec![OwnedRejection {
            locator: "data[0]".into(),
            kind: OwnedRejectionKind::InvalidContext,
            detail: "missing or malformed published person name".into(),
        }]
    );
}
