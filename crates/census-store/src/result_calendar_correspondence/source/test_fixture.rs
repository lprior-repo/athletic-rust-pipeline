use super::*;
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{AthleteCandidateId, Gender, ProfileDate, SchoolYear, Sport};
use census_domain::public_identity_history::PublicAthleteId;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: codec::digest(bytes, &mut AttachmentReplayGrantV1::new())?,
        bytes: u64::try_from(bytes.len())?,
    })
}

pub(super) fn material(bytes: &[u8]) -> TestResult<Material> {
    let reference = reference(bytes)?;
    Ok(Material::introduce(
        &AttachmentContextObjectV1::Inline {
            reference: &reference,
            bytes,
        },
        &mut AttachmentReplayGrantV1::new(),
    )?)
}

pub(super) fn source(body: &[u8]) -> TestResult<ResultCalendarSourceClaimV1> {
    let object = reference(b"controlled structural object")?;
    let digest = object.digest.clone();
    Ok(ResultCalendarSourceClaimV1 {
        result: ResultCalendarRowClaimV1 {
            result_root: object.clone(),
            body: reference(body)?,
            interpretation: object.clone(),
            row_locator: digest.clone(),
            row_digest: digest.clone(),
        },
        calendar: ResultCalendarEntryClaimV1 {
            documentary_root: object.clone(),
            capture: digest.clone(),
            interpretation: digest.clone(),
            locator: digest.clone(),
            scope: AttachmentTextV1::parse("controlled protocol scope")?,
        },
        parser_contract: reference(RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1)?,
        evidence: AttachmentListV1::new(Vec::new())?,
        person: target(digest.clone())?,
        result_attachment: record("controlled-result", &digest)?,
        documentary_attachment: record("controlled-documentary", &digest)?,
        attachment_inventory: AttachmentInventoryV1::empty()?,
        day: ProfileDate::parse("2026-03-11")?,
        sport: Sport::IndoorTrack,
        category: Gender::Girls,
        year: SchoolYear::new(2025).ok_or("school year")?,
    })
}

pub(super) fn subject(
    source: &ResultCalendarSourceClaimV1,
) -> TestResult<RetainedDocumentSubjectReferenceV1> {
    Ok(RetainedDocumentSubjectReferenceV1 {
        namespace: AttachmentTextV1::parse("tfrrs_result_document_v1")?,
        native_id: None,
        capture: source.result.result_root.digest.clone(),
        body: source.result.body.digest.clone(),
        interpretation: source.result.interpretation.digest.clone(),
        locator: source.result.row_locator.clone(),
        subject: source.result.row_digest.clone(),
        context: source.result.row_digest.clone(),
        role: AttachmentSubjectRoleV1::ResultRow,
    })
}

fn target(digest: AttachmentDigestV1) -> TestResult<AttachmentTargetV1> {
    Ok(AttachmentTargetV1 {
        public_id: PublicAthleteId::parse(
            "ath_public_v1_2e4fb039913a48c1fc1c41f2308f4ffc47f0e2b356e2cf14887893066269fa60",
        )?,
        canonical_candidate: AthleteCandidateId::mint("ath", &["controlled-person"]),
        accepted_group_digest: digest,
    })
}

fn record(operation: &str, digest: &AttachmentDigestV1) -> TestResult<AttachmentRecordReferenceV1> {
    Ok(AttachmentRecordReferenceV1 {
        operation: AttachmentTextV1::parse(operation)?,
        digest: digest.clone(),
    })
}
