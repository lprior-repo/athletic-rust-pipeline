use super::*;

pub(super) fn digest(label: &str) -> TestResult<ResultCalendarDigestV1> {
    Ok(ResultCalendarDigestV1::of(label)?)
}

fn text(value: &str) -> TestResult<ResultCalendarTextV1> {
    Ok(ResultCalendarTextV1::parse(value)?)
}

fn object(label: &str) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: digest(label)?,
        bytes: 100,
    })
}

pub(super) fn stamp() -> TestResult<chrono::DateTime<chrono::Utc>> {
    Ok(chrono::DateTime::parse_from_rfc3339("2026-10-18T00:00:00Z")?.to_utc())
}

pub(super) fn command(operation: &str) -> TestResult<ResultCalendarReviewCommandV1> {
    let store = StoreIdentity::parse(&"a".repeat(64))?;
    let initial = initial_result_calendar_inventory_v1(
        &store,
        &digest("enrollment")?,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    Ok(ResultCalendarReviewCommandV1 {
        version: 1,
        policy: text(RESULT_CALENDAR_POLICY_V1)?,
        store,
        operation: text(operation)?,
        input_token: digest("input")?,
        evidence_generation: 1,
        derived_generation: 0,
        accepted_head: AttachmentPublicHeadV1 {
            sequence: 1,
            digest: digest("head")?,
        },
        expected_inventory: initial,
        source: source()?,
        action: ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        actor: ResultCalendarReviewActorV1::LocalOperator,
        reviewed_at: stamp()?,
        rationale: text("Controlled structural review; no production source admission")?,
    })
}

fn source() -> TestResult<ResultCalendarSourceClaimV1> {
    let result = ResultCalendarRowClaimV1 {
        result_root: object("result")?,
        body: object("body")?,
        interpretation: object("row-interpretation")?,
        row_locator: digest("row-locator")?,
        row_digest: digest("original-row")?,
    };
    let calendar = ResultCalendarEntryClaimV1 {
        documentary_root: object("calendar")?,
        capture: digest("calendar-capture")?,
        interpretation: digest("calendar-interpretation")?,
        locator: digest("calendar-locator")?,
        scope: text("original-game-4375")?,
    };
    let parser_contract = object("parser")?;
    let evidence = AttachmentListV1::new(vec![
        result.result_root.clone(),
        result.body.clone(),
        result.interpretation.clone(),
        calendar.documentary_root.clone(),
        parser_contract.clone(),
    ])?;
    Ok(ResultCalendarSourceClaimV1 {
        result,
        calendar,
        parser_contract,
        evidence,
        person: AttachmentTargetV1 {
            public_id: PublicAthleteId::parse(
                "ath_public_v1_2e4fb039913a48c1fc1c41f2308f4ffc47f0e2b356e2cf14887893066269fa60",
            )?,
            canonical_candidate: AthleteCandidateId::mint("ath", &["controlled-person"]),
            accepted_group_digest: digest("group")?,
        },
        result_attachment: AttachmentRecordReferenceV1 {
            operation: text("result-attachment")?,
            digest: digest("result-record")?,
        },
        documentary_attachment: AttachmentRecordReferenceV1 {
            operation: text("status-attachment")?,
            digest: digest("status-record")?,
        },
        attachment_inventory: AttachmentInventoryV1::empty()?,
        day: ProfileDate::parse("2026-05-15")?,
        sport: Sport::OutdoorTrack,
        category: Gender::Boys,
        year: SchoolYear::new(2025).ok_or("year")?,
    })
}

pub(super) fn record(
    command: ResultCalendarReviewCommandV1,
) -> TestResult<ResultCalendarReviewedRecordV1> {
    let mut grant = AttachmentReplayGrantV1::new();
    let checked = CheckedResultCalendarCommandV1::check(command, &mut grant)?;
    let approval = result_calendar_approval_bytes_v1(&checked, &mut grant)?;
    let value = prepare_result_calendar_record_v1(checked, stamp()?, &mut grant)?;
    check!(eq; value.approval_digest, digest_bytes_for_test(&approval)?);
    Ok(value)
}

fn digest_bytes_for_test(bytes: &[u8]) -> TestResult<ResultCalendarDigestV1> {
    use sha2::{Digest, Sha256};
    Ok(ResultCalendarDigestV1::parse(&format!(
        "{:x}",
        Sha256::digest(bytes)
    ))?)
}

pub(super) fn receipt(value: &ResultCalendarReviewedRecordV1) -> ResultCalendarReceiptBindingV1 {
    ResultCalendarReceiptBindingV1 {
        operation: value.command.operation.clone(),
        record_digest: value.record_digest.clone(),
    }
}

pub(super) fn next(
    value: &ResultCalendarReviewedRecordV1,
) -> TestResult<ResultCalendarInventoryV1> {
    Ok(next_result_calendar_inventory_v1(
        &value.command.expected_inventory,
        value,
        &mut AttachmentReplayGrantV1::new(),
    )?)
}
