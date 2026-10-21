use super::*;
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use chrono::{DateTime, Utc};

pub(in crate::participation_evidence) type TestResult<T = ()> =
    Result<T, Box<dyn std::error::Error>>;
pub(in crate::participation_evidence) const REVIEW: &str =
    include_str!("../../roncalli/tests/fixtures/season-review.html");
pub(in crate::participation_evidence) const SCHEDULE: &str =
    include_str!("../../roncalli/tests/fixtures/schedule.html");
const REVIEW_URL: &str = "https://athletics.roncalli.org/girls-track-field/";
const SCHEDULE_URL: &str = "https://athletics.roncalli.org/girls-track-field/schedules/";

mod guards;

pub(in crate::participation_evidence) fn capture(
    url: &str,
    body: &[u8],
) -> TestResult<RetainedHtmlCaptureV1> {
    let time = DateTime::parse_from_rfc3339("2026-10-21T12:00:00Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        time,
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "controlled-transport-test/1".into()),
                ("Accept".into(), "text/html".into()),
            ],
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=utf-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: time,
        },
    )?)
}

pub(in crate::participation_evidence) fn selection() -> RoncalliSeasonEventSelectionV1 {
    RoncalliSeasonEventSelectionV1 {
        status_subject_start: 76865,
        status_subject_end: 76882,
        calendar_recap_id: "event-3387-recap".into(),
    }
}

pub(in crate::participation_evidence) fn packet(
    review: &str,
    schedule: &str,
) -> TestResult<IndependentParticipationPacketV1> {
    Ok(
        IndependentParticipationPacketV1::from_roncalli_season_event(
            capture(REVIEW_URL, review.as_bytes())?,
            capture(SCHEDULE_URL, schedule.as_bytes())?,
            selection(),
        )?,
    )
}

pub(in crate::participation_evidence) fn read(
    packet: &IndependentParticipationPacketV1,
) -> TestResult<VerifiedIndependentParticipationV1> {
    Ok(packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7)?)
}

#[test]
fn genuine_pair_derives_only_selected_event_day_and_retains_school_only_finish_place() -> TestResult
{
    let packet = packet(REVIEW, SCHEDULE)?;
    let source = read(&packet)?;
    check!(eq; source.subject().document_namespace(), "roncalli_girls_track_season_review_v1");
    check!(eq; source.subject().native_document_id(), "girls-track-field:2026:76865:76882");
    check!(eq; source.subject().locator(), "html_bytes:76865:76882");
    check!(eq; source.subject().name(), "Tessa Mize");
    check!(eq; source.subject().school(), "Roncalli");
    check!(eq; source.day().as_str(), "2026-03-11");
    check!(eq; source.school_year().get(), 2025);
    check!(eq; source.cohort().get(), 2027);
    check!(eq; source.competition(), "at HSR Indoor Meet");
    check!(eq; source.venue(), "UIndy");
    check!(eq; source.season_scope(), format!("{SCHEDULE_URL}#event-3387-recap"));
    check!(eq; source.status_proof().capture(), hash(REVIEW.as_bytes()));
    check!(eq; source.season_proof().capture(), hash(SCHEDULE.as_bytes()));
    check!(eq; source.status_proof().locator(), source.subject().locator());
    check!(eq; source.status_proof().interpretation(), source.season_proof().interpretation());
    let checked = source.roncalli_event().ok_or("checked candidate absent")?;
    check!(eq; checked.event().place, 2);
    check!(eq; checked.event().original_mark, "11:29.42");
    check!(eq; checked.event().original_centiseconds, 68942);
    check!(
        matches!(source.ownership(), SubjectOwnershipV1::DocumentOnly(document) if document == source.subject())
    );
    Ok(())
}

#[test]
fn packet_serialization_reopens_same_pair_proofs_and_selector() -> TestResult {
    let packet = packet(REVIEW, SCHEDULE)?;
    let original = read(&packet)?;
    let bytes = packet.bytes()?;
    let reopened: IndependentParticipationPacketV1 = serde_json::from_slice(&bytes)?;
    let current = read(&reopened)?;
    check!(eq; current.reference(), original.reference());
    check!(eq; current.status_proof().interpretation(), original.status_proof().interpretation());
    check!(eq; current.season_proof().locator(), original.season_proof().locator());
    check!(eq; current.roncalli_event(), original.roncalli_event());
    check!(eq; reopened.bytes()?, bytes);
    Ok(())
}

#[test]
fn selected_calendar_day_changes_both_proofs_without_backdating_the_season_document() -> TestResult
{
    let original = read(&packet(REVIEW, SCHEDULE)?)?;
    let changed = SCHEDULE.replace("Wed Mar 11", "Thu Mar 12");
    let current = read(&packet(REVIEW, &changed)?)?;
    check!(eq; current.day().as_str(), "2026-03-12");
    check!(eq; current.status_proof().capture(), original.status_proof().capture());
    check!(ne; current.status_proof().interpretation(), original.status_proof().interpretation());
    check!(ne; current.season_proof().capture(), original.season_proof().capture());
    check!(ne; current.reference(), original.reference());
    Ok(())
}
