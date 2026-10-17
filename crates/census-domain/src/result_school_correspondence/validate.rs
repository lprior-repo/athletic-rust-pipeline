use super::*;
use crate::evidence_subject_attachment::AttachmentObjectReferenceV1;
use crate::model::{Gender, SourceNamespace, Sport};

pub(super) fn command(
    value: &ResultSchoolReviewCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    encode(value, grant)?;
    if value.version != 1
        || value.policy.as_str() != RESULT_SCHOOL_POLICY_V1
        || value.expected_inventory.version != 1
        || value.operation.as_str().len() > 256
        || value.expected_inventory.records
            >= u64::try_from(MAX_RESULT_SCHOOL_RECORDS)
                .map_err(|_| ResultSchoolCorrespondenceError::Capacity)?
        || !value.source.person.canonical_candidate.validate("ath")
    {
        return Err(ResultSchoolCorrespondenceError::Invalid(
            "command schema or identity",
        ));
    }
    statement(&value.source.statement, grant)?;
    target(&value.source.target)?;
    if value.source.person_inventory.version != 1
        || value.source.evidence.is_empty()
        || value.source.evidence.len() > MAX_RESULT_SCHOOL_OBJECTS
    {
        return Err(ResultSchoolCorrespondenceError::Invalid(
            "evidence inventory",
        ));
    }
    let count = value.source.evidence.len();
    let pairs = count
        .checked_mul(count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(ResultSchoolCorrespondenceError::Capacity)?;
    grant.charge_steps(pairs)?;
    for (index, item) in value.source.evidence.as_slice().iter().enumerate() {
        grant.charge_steps(1)?;
        object(item)?;
        if value
            .source
            .evidence
            .as_slice()
            .get(..index)
            .ok_or(ResultSchoolCorrespondenceError::Capacity)?
            .iter()
            .any(|previous| previous.digest == item.digest)
        {
            return Err(ResultSchoolCorrespondenceError::Invalid(
                "duplicate evidence object",
            ));
        }
    }
    Ok(())
}

fn statement(
    value: &ResultSchoolStatementClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    grant.charge_steps(1)?;
    for item in [
        &value.result_root,
        &value.body,
        &value.interpretation,
        &value.parser_contract,
    ] {
        object(item)?;
    }
    let row_end = value
        .row_offset
        .checked_add(value.row_length)
        .ok_or(ResultSchoolCorrespondenceError::Capacity)?;
    let statement_end = value
        .statement_offset
        .checked_add(value.statement_length)
        .ok_or(ResultSchoolCorrespondenceError::Capacity)?;
    if value.row_length == 0
        || value.statement_length == 0
        || value.statement_offset < value.row_offset
        || statement_end > row_end
        || row_end > value.body.bytes
        || value.statement_text.as_str().len() > 512
        || u64::try_from(value.statement_text.as_str().len())
            .map_err(|_| ResultSchoolCorrespondenceError::Capacity)?
            != value.statement_length
        || digest_bytes(value.statement_text.as_str().as_bytes())? != value.statement_digest
    {
        return Err(ResultSchoolCorrespondenceError::Invalid(
            "original school statement span",
        ));
    }
    Ok(())
}

fn target(value: &ResultSchoolTargetClaimV1) -> Result<()> {
    for item in [
        &value.provider_school_root,
        &value.physical_school_inventory,
        &value.year_contract,
    ] {
        object(item)?;
    }
    if !value.school.validate("sch")
        || *value.source_school.namespace() != SourceNamespace::MilesplitSchool
        || !matches!(
            value.sport,
            Sport::OutdoorTrack | Sport::IndoorTrack | Sport::CrossCountry
        )
        || !matches!(value.category, Gender::Boys | Gender::Girls)
    {
        return Err(ResultSchoolCorrespondenceError::Invalid(
            "school target scope",
        ));
    }
    Ok(())
}

fn object(value: &AttachmentObjectReferenceV1) -> Result<()> {
    if value.bytes == 0 || value.bytes > 4 * 1024 * 1024 {
        return Err(ResultSchoolCorrespondenceError::Capacity);
    }
    Ok(())
}

pub(super) fn record(
    value: &ResultSchoolReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    command(&value.command, grant)?;
    if value.prepared_digest != result_school_digest_v1(&value.command, grant)?
        || value.approval_digest
            != digest_bytes(&result_school_approval_bytes_v1(&value.command, grant)?)?
        || value.record_digest != record_digest(value, grant)?
        || value.committed_at < value.command.reviewed_at
    {
        return Err(ResultSchoolCorrespondenceError::Invalid(
            "record digest or time",
        ));
    }
    Ok(())
}

pub(super) fn record_digest(
    value: &ResultSchoolReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolDigestV1> {
    result_school_digest_v1(
        &(
            "result-school-reviewed-record-v1",
            &value.command,
            &value.prepared_digest,
            &value.approval_digest,
            value.committed_at,
        ),
        grant,
    )
}
