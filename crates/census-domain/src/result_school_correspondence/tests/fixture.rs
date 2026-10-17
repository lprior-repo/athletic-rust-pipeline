use super::*;

pub(super) fn digest(value: &str) -> TestResult<ResultSchoolDigestV1> {
    Ok(ResultSchoolDigestV1::of(value)?)
}
pub(super) fn text(value: &str) -> TestResult<ResultSchoolTextV1> {
    Ok(ResultSchoolTextV1::parse(value)?)
}
pub(super) fn object(value: &str) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: digest(value)?,
        bytes: 100,
    })
}
pub(super) fn stamp() -> TestResult<chrono::DateTime<chrono::Utc>> {
    Ok(chrono::DateTime::parse_from_rfc3339("2026-10-17T00:00:00Z")?.to_utc())
}
pub(super) fn command(operation: &str) -> TestResult<ResultSchoolReviewCommandV1> {
    let store = StoreIdentity::parse(&"a".repeat(64))?;
    let initial = initial_result_school_inventory_v1(
        &store,
        &digest("controlled-enrollment-basis")?,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    Ok(ResultSchoolReviewCommandV1 {
        version: 1, policy: text(RESULT_SCHOOL_POLICY_V1)?, store, operation: text(operation)?, input_token: digest("input-token")?, evidence_generation: 1, derived_generation: 0,
        accepted_head: AttachmentPublicHeadV1 { sequence: 1, digest: digest("public-head")? }, expected_inventory: initial,
        source: ResultSchoolSourceClaimV1 {
            statement: ResultSchoolStatementClaimV1 { result_root: object("result-root")?, body: object("body")?, interpretation: object("interpretation")?, row_locator: digest("locator")?, row_digest: digest("original-row")?, row_offset: 0, row_length: 100, statement_offset: 20, statement_length: 10, statement_digest: raw_digest("Controlled")?, statement_text: text("Controlled")?, parser_contract: object("parser")? },
            target: ResultSchoolTargetClaimV1 { school: SchoolId::mint("sch", &["controlled-school"]), source_school: SourceSubject::new(SourceNamespace::MilesplitSchool, "42".into())?, provider_school_root: object("school-root")?, physical_school_inventory: object("physical-schools")?, day: ProfileDate::parse("2026-05-15")?, sport: Sport::OutdoorTrack, category: Gender::Boys, year: SchoolYear::new(2025).ok_or("controlled year")?, year_contract: object("year-policy")? },
            person: AttachmentTargetV1 { public_id: PublicAthleteId::parse("ath_public_v1_2e4fb039913a48c1fc1c41f2308f4ffc47f0e2b356e2cf14887893066269fa60")?, canonical_candidate: AthleteCandidateId::mint("ath", &["controlled-person"]), accepted_group_digest: digest("accepted-group")? },
            person_attachment: AttachmentRecordReferenceV1 { operation: text("controlled-person-attachment")?, digest: digest("person-record")? }, person_inventory: AttachmentInventoryV1::empty()?, evidence: AttachmentListV1::new(vec![object("controlled-evidence")?])?,
        }, action: ResultSchoolActionV1::Accept { prior: ResultSchoolPredecessorV1::First }, actor: ResultSchoolReviewActorV1::LocalOperator, reviewed_at: stamp()?, rationale: text("Controlled structural model only; no human production approval or source school admission")?,
    })
}

pub(super) fn record(
    command: ResultSchoolReviewCommandV1,
) -> TestResult<ResultSchoolReviewedRecordV1> {
    let mut grant = AttachmentReplayGrantV1::new();
    let checked = CheckedResultSchoolCommandV1::check(command, &mut grant)?;
    Ok(prepare_result_school_record_v1(
        checked,
        stamp()?,
        &mut grant,
    )?)
}

pub(super) fn receipt(record: &ResultSchoolReviewedRecordV1) -> ResultSchoolReceiptBindingV1 {
    ResultSchoolReceiptBindingV1 {
        operation: record.command.operation.clone(),
        record_digest: record.record_digest.clone(),
    }
}

pub(super) fn next(record: &ResultSchoolReviewedRecordV1) -> TestResult<ResultSchoolInventoryV1> {
    Ok(next_result_school_inventory_v1(
        &record.command.expected_inventory,
        record,
        &mut AttachmentReplayGrantV1::new(),
    )?)
}

pub(super) fn raw_digest(value: &str) -> TestResult<ResultSchoolDigestV1> {
    use sha2::{Digest, Sha256};
    Ok(ResultSchoolDigestV1::parse(&format!(
        "{:x}",
        Sha256::digest(value.as_bytes())
    ))?)
}
