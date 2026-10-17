use super::*;

pub(super) fn references(
    evidence: &RosterSourceEvidenceV1,
    binding: &RosterExistingBindingClaimV1,
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    evidence.require_original_grant(grant)?;
    projection_reference(school, binding.school_projection(), grant)?;
    evidence.require_original_grant(grant)?;
    projection_reference(program, binding.program_projection(), grant)?;
    evidence.require_original_grant(grant)?;
    let manifest = evidence.physical_projection_manifest(grant)?;
    require_reference(&manifest, binding.physical_projection_inventory(), grant)?;
    evidence.require_original_grant(grant)
}

pub(super) fn projection_reference(
    value: &impl serde::Serialize,
    expected: &RosterAuthorityObjectRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    budget::charge(value, grant)?;
    let bytes = serde_json::to_vec(value).map_err(json)?;
    require_reference(&bytes, expected, grant)
}

pub(super) fn require_reference(
    bytes: &[u8],
    expected: &RosterAuthorityObjectRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(domain)?;
    grant.charge_bytes(bytes.len()).map_err(domain)?;
    let length = u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?;
    if expected.bytes().get() != length || expected.digest().as_str() != hash(bytes) {
        return Err(invalid(
            "roster binding canonical object reference mismatch",
        ));
    }
    Ok(())
}

pub(super) fn scan<T: Entity>(
    evidence: &RosterSourceEvidenceV1,
    table: Table,
    grant: &mut AttachmentReplayGrantV1,
    mut inspect: impl FnMut(&T) -> StoreResult<()>,
    mut visit: impl FnMut(T) -> StoreResult<()>,
) -> StoreResult<()> {
    let rows = evidence.physical_projection(grant)?;
    let mut current: Option<T> = None;
    let mut previous: Option<&[u8]> = None;
    for physical in rows.iter().filter(|row| row.table() == table) {
        evidence.require_original_grant(grant)?;
        if previous.is_some_and(|key| key >= physical.key()) {
            return Err(invalid("roster binding physical key order differs"));
        }
        previous = Some(physical.key());
        grant.charge_bytes(physical.bytes().len()).map_err(domain)?;
        grant.charge_steps(1).map_err(domain)?;
        let row: T = decode_physical_observation(table, physical.key(), physical.bytes())?;
        inspect(&row)?;
        evidence.require_original_grant(grant)?;
        if let Some(existing) = current
            .as_mut()
            .filter(|item| item.entity_id() == row.entity_id())
        {
            budget::charge(existing, grant)?;
            budget::charge(&row, grant)?;
            existing.merge(row);
            evidence.require_original_grant(grant)?;
        } else if let Some(prior) = current.replace(row) {
            publish(evidence, prior, grant, &mut inspect, &mut visit)?;
        }
    }
    if let Some(last) = current {
        publish(evidence, last, grant, &mut inspect, &mut visit)?;
    }
    evidence.require_original_grant(grant)
}

pub(super) fn publish<T: Entity>(
    evidence: &RosterSourceEvidenceV1,
    mut row: T,
    grant: &mut AttachmentReplayGrantV1,
    inspect: &mut impl FnMut(&T) -> StoreResult<()>,
    visit: &mut impl FnMut(T) -> StoreResult<()>,
) -> StoreResult<()> {
    evidence.require_original_grant(grant)?;
    budget::charge(&row, grant)?;
    row.publish();
    inspect(&row)?;
    visit(row)?;
    evidence.require_original_grant(grant)
}

pub(super) fn domain(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> StoreError {
    census_store::EvidenceSubjectAttachmentStoreError::Domain(error).into()
}
