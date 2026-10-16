use super::*;
use crate::evidence_subject_attachment::ledger::Ledger;
use census_domain::evidence_subject_attachment::{
    AttachmentPublicHeadV1, ReviewedReferenceRecordV1,
};
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness<'a> {
    #[serde(borrow)]
    schema: &'a str,
    store: StoreIdentity,
    #[serde(borrow)]
    qualification: &'a str,
    public_head: AttachmentPublicHeadV1,
    evidence_generation: u64,
    derived_generation: u64,
    group: AttachmentObjectReferenceV1,
    capture_binding: AttachmentObjectReferenceV1,
    input_revisions: AttachmentObjectReferenceV1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group<'a> {
    #[serde(borrow)]
    schema: &'a str,
    store: StoreIdentity,
    #[serde(borrow)]
    qualification: &'a str,
    #[serde(borrow)]
    public_id: &'a str,
    #[serde(borrow)]
    representative: &'a str,
    established: Position,
    #[serde(borrow)]
    claims: Vec<&'a RawValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    sequence: u64,
    position: u32,
}

pub(in super::super) fn verify(
    ledger: &Ledger<'_>,
    record: &ReviewedReferenceRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let raw = ledger
        .object(
            &record.command.reviewed_context.accepted_target_witness,
            grant,
        )?
        .inline_bytes(grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if raw.first() != Some(&b'{') {
        return Err(Error::Corrupt("historical witness must be an object").into());
    }
    let witness: Witness<'_> =
        serde_json::from_slice(&raw).map_err(|source| Error::Domain(source.into()))?;
    encoding::require_canonical(&witness, &raw, grant)?;
    if witness.schema != "retained-evidence-subject-attachment-target-witness/v1"
        || witness.qualification != "retained_authority_graph_v3"
        || witness.store != record.command.store
    {
        return Err(Error::Corrupt("historical target witness identity").into());
    }
    require_origin(ledger, record, &witness, grant)?;
    for reference in [
        &witness.group,
        &witness.capture_binding,
        &witness.input_revisions,
    ] {
        require_declared(record, reference, grant)?;
    }
    let group = ledger
        .object(&witness.group.digest, grant)?
        .inline_bytes(grant)?;
    require_group(&group, record, &witness.group, grant)?;
    let capture = ledger
        .object(&witness.capture_binding.digest, grant)?
        .inline_bytes(grant)?;
    grant.charge_steps(3).map_err(Error::Domain)?;
    let capture =
        crate::public_identity::paged_generation::decode_attachment_capture_binding(&capture)?;
    let token = ledger
        .object(&witness.input_revisions.digest, grant)?
        .inline_bytes(grant)?;
    grant.charge_steps(5).map_err(Error::Domain)?;
    crate::public_identity::paged_generation::require_attachment_retained_inputs(
        &token,
        &capture,
        &witness.store,
        witness.evidence_generation,
        witness.derived_generation,
    )?;
    Ok(())
}

fn require_origin(
    ledger: &Ledger<'_>,
    record: &ReviewedReferenceRecordV1,
    witness: &Witness<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for original in &ledger.records {
        grant.charge_steps(1).map_err(Error::Domain)?;
        if original.command.reviewed_context.accepted_target_witness
            != record.command.reviewed_context.accepted_target_witness
        {
            continue;
        }
        if witness.public_head != original.command.expected_public_head
            || witness.evidence_generation != original.command.expected_evidence_generation
            || witness.derived_generation != original.command.expected_derived_generation
        {
            return Err(Error::Corrupt("historical target witness origin fence").into());
        }
        return Ok(());
    }
    Err(Error::Corrupt("historical target witness has no origin").into())
}

fn require_declared(
    record: &ReviewedReferenceRecordV1,
    reference: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if super::super::ledger::declared(
        record.command.reviewed_context.objects.as_slice(),
        reference,
        grant,
    )? {
        return Ok(());
    }
    Err(Error::Corrupt("historical target witness dependency omitted").into())
}

fn require_group(
    raw: &[u8],
    record: &ReviewedReferenceRecordV1,
    reference: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if raw.first() != Some(&b'{') {
        return Err(Error::Corrupt("historical group must be an object").into());
    }
    let group: Group<'_> =
        serde_json::from_slice(raw).map_err(|source| Error::Domain(source.into()))?;
    encoding::require_canonical(&group, raw, grant)?;
    if group.schema != "retained-evidence-subject-attachment-group/v1"
        || group.qualification != "retained_authority_graph_v3"
        || group.store != record.command.store
        || group.public_id != record.command.target.public_id.as_str()
        || group.representative != record.command.target.canonical_candidate.as_str()
        || reference.digest != record.command.target.accepted_group_digest
        || group.established.sequence == 0
        || group.claims.is_empty()
        || group.claims.len() > ReplayLimits::SMALL.members_per_operation
    {
        return Err(Error::Corrupt("historical semantic target group").into());
    }
    let _position = group.established.position;
    super::historical_claims::validate(&group.claims, grant)
}

#[cfg(test)]
mod tests;
