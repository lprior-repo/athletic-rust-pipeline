use super::*;
use census_domain::model::{CanonicalSchool, SchoolYear};
mod provenance;

pub(super) fn require_contracts(
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    root: &crate::RetainedCaptureRootV1,
    provider: &crate::evidence_subject_attachment::external::CheckedSchool<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let parser = object(materials, &source.statement.parser_contract)?.inline_bytes(grant)?;
    let year = object(materials, &source.target.year_contract)?.inline_bytes(grant)?;
    if parser != RESULT_SCHOOL_PARSER_CONTRACT_V1
        || year != RESULT_SCHOOL_YEAR_CONTRACT_V1
        || SchoolYear::from_date(source.target.day.as_str()) != Some(source.target.year)
    {
        return Err(Error::SourceMismatch("parser/year contract").into());
    }
    let physical =
        object(materials, &source.target.physical_school_inventory)?.inline_bytes(grant)?;
    let school: CanonicalSchool = codec::decode(&physical, grant)?;
    if school.id != source.target.school
        || !school.source_identities.iter().any(|identity| {
            identity.namespace == *source.target.source_school.namespace()
                && identity.id == source.target.source_school.provider_id()
        })
    {
        return Err(Error::SourceMismatch("school provider identity").into());
    }
    provenance::require(&school, root, provider, grant)
}

pub(super) fn require_installed(
    view: &StoreSnapshot<'_>,
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant
        .charge_bytes(MAX_RESULT_SCHOOL_RECORD_BYTES)
        .map_err(grant_error)?;
    grant.charge_steps(1).map_err(grant_error)?;
    let school: CanonicalSchool = view
        .get_merged(crate::Table::Schools, source.target.school.as_str())?
        .ok_or(Error::SourceMismatch("installed school absent"))?;
    let actual = codec::encode(&school, grant)?;
    let proposed =
        object(materials, &source.target.physical_school_inventory)?.inline_bytes(grant)?;
    if actual != proposed {
        return Err(Error::SourceMismatch("installed school inventory").into());
    }
    Ok(())
}
