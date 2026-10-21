use super::*;
use crate::result_calendar_correspondence::source::test_fixture::*;
use census_domain::evidence_subject_attachment::AttachmentTextV1;
use census_domain::model::ProfileDate;

const STATUS: &[u8] = b"<b>Controlled Junior Person</b>";
const CALENDAR: &[u8] = b"<div id='event-3387-recap'>Controlled calendar day</div>";
const STATUS_LOCATOR: &[u8] = b"html_bytes:0:31";
const CALENDAR_LOCATOR: &[u8] = b"html_bytes:0:56";

type ProofFixture = (
    ResultCalendarSourceClaimV1,
    RetainedDocumentSubjectReferenceV1,
    Vec<Material>,
);

fn selection() -> RoncalliSeasonEventSelectionV1 {
    RoncalliSeasonEventSelectionV1 {
        status_subject_start: 0,
        status_subject_end: STATUS.len(),
        calendar_recap_id: "event-3387-recap".to_owned(),
    }
}

fn fixture() -> TestResult<ProofFixture> {
    let mut source = source(b"controlled result row")?;
    source.calendar.capture = reference(CALENDAR)?.digest;
    let mut subject = subject(&source)?;
    subject.namespace = AttachmentTextV1::parse("roncalli_girls_track_season_review_v1")?;
    subject.body = reference(STATUS)?.digest;
    let bytes = selection().interpretation_preimage(RoncalliSeasonEventProofContextV1 {
        status_body_sha: subject.body.as_str(),
        schedule_body_sha: source.calendar.capture.as_str(),
        status_locator: std::str::from_utf8(STATUS_LOCATOR)?,
        calendar_locator: std::str::from_utf8(CALENDAR_LOCATOR)?,
        day: &source.day,
    });
    subject.interpretation = reference(bytes.as_bytes())?.digest;
    source.calendar.interpretation = subject.interpretation.clone();
    Ok((source, subject, vec![material(bytes.as_bytes())?]))
}

#[test]
fn exact_selected_original_spans_and_shared_proof_replay() -> TestResult {
    locators(
        &selection(),
        (&reference(STATUS)?, STATUS),
        STATUS_LOCATOR,
        (&reference(CALENDAR)?, CALENDAR),
        CALENDAR_LOCATOR,
    )?;
    let (source, subject, materials) = fixture()?;
    proof(
        &source,
        &materials,
        &subject,
        &selection(),
        (STATUS_LOCATOR, CALENDAR_LOCATOR),
        &mut AttachmentReplayGrantV1::new(),
    )?;
    let encoded = serde_json::to_vec(&materials)?;
    let cold: Vec<Material> = serde_json::from_slice(&encoded)?;
    proof(
        &source,
        &cold,
        &subject,
        &selection(),
        (STATUS_LOCATOR, CALENDAR_LOCATOR),
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; serde_json::to_vec(&cold)?, encoded);
    Ok(())
}

#[test]
fn selected_subject_and_calendar_span_cannot_shift_or_use_noncanonical_locator() -> TestResult {
    for status_locator in [
        b"html_bytes:1:31".as_slice(),
        b"html_bytes:00:31",
        b"html_bytes:0:32",
    ] {
        check!(locators(
            &selection(),
            (&reference(STATUS)?, STATUS),
            status_locator,
            (&reference(CALENDAR)?, CALENDAR),
            CALENDAR_LOCATOR
        )
        .is_err());
    }
    for calendar_locator in [
        b"html_bytes:0:57".as_slice(),
        b"html_bytes:0:0",
        b"html_bytes:00:56",
    ] {
        check!(locators(
            &selection(),
            (&reference(STATUS)?, STATUS),
            STATUS_LOCATOR,
            (&reference(CALENDAR)?, CALENDAR),
            calendar_locator
        )
        .is_err());
    }
    let body = "aéz".as_bytes();
    check!(exact_span(body, b"html_bytes:1:2", (1, 2)).is_err());
    check!(exact_span(&[0xff], b"html_bytes:0:1", (0, 1)).is_err());
    Ok(())
}

#[test]
fn changed_day_recap_or_shared_proof_cannot_be_accepted_after_rehash() -> TestResult {
    let (source, subject, materials) = fixture()?;
    let mut changed = source.clone();
    changed.day = ProfileDate::parse("2026-03-12")?;
    check!(proof(
        &changed,
        &materials,
        &subject,
        &selection(),
        (STATUS_LOCATOR, CALENDAR_LOCATOR),
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    let mut changed_selection = selection();
    changed_selection.calendar_recap_id = "event-3388-recap".to_owned();
    check!(proof(
        &source,
        &materials,
        &subject,
        &changed_selection,
        (STATUS_LOCATOR, CALENDAR_LOCATOR),
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    Ok(())
}

#[test]
fn validly_rehashed_shared_proof_is_refused_with_sticky_grant() -> TestResult {
    let (source, subject, _) = fixture()?;
    let mut changed_source = source;
    let mut changed_subject = subject;
    let forged = b"validly rehashed unrelated proof";
    changed_subject.interpretation = reference(forged)?.digest;
    changed_source.calendar.interpretation = changed_subject.interpretation.clone();
    let forged_materials = [material(forged)?];
    let mut grant = AttachmentReplayGrantV1::new();
    check!(guarded(&mut grant, |grant| proof(
        &changed_source,
        &forged_materials,
        &changed_subject,
        &selection(),
        (STATUS_LOCATOR, CALENDAR_LOCATOR),
        grant
    ))
    .is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn shared_preimage_is_bounded_to_512_and_old_path_keeps_256() -> TestResult {
    let (mut source, mut subject, _) = fixture()?;
    let oversized = vec![b'x'; 513];
    subject.interpretation = reference(&oversized)?.digest;
    source.calendar.interpretation = subject.interpretation.clone();
    check!(proof(
        &source,
        &[material(&oversized)?],
        &subject,
        &selection(),
        (STATUS_LOCATOR, CALENDAR_LOCATOR),
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    let oversized_old = vec![b'x'; 257];
    let locator = b"html_bytes:0:1";
    let body = reference(STATUS)?;
    let locator_ref = reference(locator)?;
    let proof_ref = reference(&oversized_old)?;
    check!(super::super::preimage::interpretation(
        &[
            material(STATUS)?,
            material(locator)?,
            material(&oversized_old)?
        ],
        &body.digest,
        &locator_ref.digest,
        &proof_ref.digest,
        "mhsaa_subject_dcc_event_day_v1",
        &mut AttachmentReplayGrantV1::new()
    )
    .is_err());
    Ok(())
}
