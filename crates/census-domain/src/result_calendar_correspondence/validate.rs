use super::*;
use crate::evidence_subject_attachment::{AttachmentListV1, AttachmentObjectReferenceV1};
use crate::model::{Gender, SchoolYear, Sport};

pub(super) fn command(
    value: &ResultCalendarReviewCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    schema(value)?;
    source(&value.source, grant)?;
    admit(value, grant)
}

fn schema(value: &ResultCalendarReviewCommandV1) -> Result<()> {
    if value.version != 1
        || value.policy.as_str() != RESULT_CALENDAR_POLICY_V1
        || value.expected_inventory.version != 1
        || value.operation.as_str().len() > 256
        || value.expected_inventory.records
            >= u64::try_from(MAX_RESULT_CALENDAR_RECORDS)
                .map_err(|_| ResultCalendarCorrespondenceError::Capacity)?
        || !value.source.person.canonical_candidate.validate("ath")
    {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "command schema or identity",
        ));
    }
    Ok(())
}

fn source(value: &ResultCalendarSourceClaimV1, grant: &mut AttachmentReplayGrantV1) -> Result<()> {
    result_claim(&value.result, grant)?;
    calendar_claim(&value.calendar, grant)?;
    if value.attachment_inventory.version != 1
        || value.evidence.is_empty()
        || value.evidence.len() > MAX_RESULT_CALENDAR_OBJECTS
    {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "evidence inventory",
        ));
    }
    evidence(&value.evidence, grant)?;
    for required in [
        &value.result.result_root,
        &value.result.body,
        &value.result.interpretation,
        &value.calendar.documentary_root,
        &value.parser_contract,
    ] {
        grant.charge_steps(value.evidence.len())?;
        evidence_contains(&value.evidence, required)?;
    }
    if value.result.result_root == value.calendar.documentary_root {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "self-witness result and calendar",
        ));
    }
    scope(value)
}

fn evidence(
    value: &AttachmentListV1<AttachmentObjectReferenceV1>,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    let count = value.len();
    let pairs = count
        .checked_mul(count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(ResultCalendarCorrespondenceError::Capacity)?;
    grant.charge_steps(pairs)?;
    for (index, item) in value.as_slice().iter().enumerate() {
        grant.charge_steps(1)?;
        object(item)?;
        if value
            .as_slice()
            .get(..index)
            .ok_or(ResultCalendarCorrespondenceError::Capacity)?
            .iter()
            .any(|previous| previous.digest == item.digest)
        {
            return Err(ResultCalendarCorrespondenceError::Invalid(
                "duplicate evidence object",
            ));
        }
    }
    Ok(())
}

fn scope(value: &ResultCalendarSourceClaimV1) -> Result<()> {
    if !matches!(
        value.sport,
        Sport::OutdoorTrack | Sport::IndoorTrack | Sport::CrossCountry
    ) || !matches!(value.category, Gender::Boys | Gender::Girls)
    {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "calendar sport or category scope",
        ));
    }
    let year = SchoolYear::from_date(value.day.as_str()).ok_or(
        ResultCalendarCorrespondenceError::Invalid("calendar day date parse"),
    )?;
    if year != value.year {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "calendar day and year disagreement",
        ));
    }
    Ok(())
}

fn result_claim(
    value: &ResultCalendarRowClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    grant.charge_steps(1)?;
    for item in [&value.result_root, &value.body, &value.interpretation] {
        object(item)?;
    }
    Ok(())
}

fn calendar_claim(
    value: &ResultCalendarEntryClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    grant.charge_steps(1)?;
    object(&value.documentary_root)?;
    Ok(())
}

fn evidence_contains(
    evidence: &AttachmentListV1<AttachmentObjectReferenceV1>,
    required: &AttachmentObjectReferenceV1,
) -> Result<()> {
    let found = evidence
        .as_slice()
        .iter()
        .find(|item| item.digest == required.digest);
    match found {
        Some(item) if item.bytes != required.bytes => Err(
            ResultCalendarCorrespondenceError::Invalid("evidence length mismatch"),
        ),
        Some(_) => Ok(()),
        None => Err(ResultCalendarCorrespondenceError::Invalid(
            "required object not in evidence",
        )),
    }
}

pub(super) fn record(
    value: &ResultCalendarReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    command(&value.command, grant)?;
    if value.prepared_digest != result_calendar_digest_v1(&value.command, grant)?
        || value.approval_digest != approval_digest(&value.command, grant)?
        || value.record_digest != record_digest(value, grant)?
        || value.committed_at < value.command.reviewed_at
    {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "record digest or time",
        ));
    }
    Ok(())
}

pub(super) fn record_digest(
    value: &ResultCalendarReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarDigestV1> {
    result_calendar_digest_v1(
        &(
            "result-calendar-reviewed-record-v1",
            &value.command,
            &value.prepared_digest,
            &value.approval_digest,
            value.committed_at,
        ),
        grant,
    )
}

fn object(value: &AttachmentObjectReferenceV1) -> Result<()> {
    if value.bytes == 0 || value.bytes > 4 * 1024 * 1024 {
        return Err(ResultCalendarCorrespondenceError::Capacity);
    }
    Ok(())
}
