use super::{Error, RosterSourceEvidenceV1};
use crate::{AcquisitionReadBudget, PublicSchoolIntakeAdmissionV1, PublicSchoolIntakePacketV1};
use crate::{RetainedCaptureRootSchemaV1 as Schema, StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::roster_source_issuance) struct ObservedIntakePins {
    mint: AttachmentObjectReferenceV1,
    receipt: AttachmentObjectReferenceV1,
}

impl ObservedIntakePins {
    pub(in crate::roster_source_issuance) fn references(
        &self,
    ) -> [&AttachmentObjectReferenceV1; 2] {
        [&self.mint, &self.receipt]
    }
}

pub(in crate::roster_source_issuance) fn admission_kind(schema: Schema) -> &'static str {
    match schema {
        Schema::ProviderSchoolObservedPacketV1 => {
            "completed-exact-owned-observed-provider-school-root"
        }
        _ => "completed-exact-provider-school-root",
    }
}

pub(in crate::roster_source_issuance) fn authorization(schema: Schema) -> &'static str {
    match schema {
        Schema::ProviderSchoolObservedPacketV1 => "qualified-owned-observed-acquisition",
        _ => "unknown",
    }
}

pub(in crate::roster_source_issuance) fn descriptor_matches(
    schema: Schema,
    kind: &str,
    authorization_value: &str,
    has_pins: bool,
) -> bool {
    matches!(
        schema,
        Schema::ProviderSchoolPacketV1 | Schema::ProviderSchoolObservedPacketV1
    ) && kind == admission_kind(schema)
        && authorization_value == authorization(schema)
        && has_pins == (schema == Schema::ProviderSchoolObservedPacketV1)
}

pub(in crate::roster_source_issuance) fn pins(
    proof: &PublicSchoolIntakeAdmissionV1,
) -> StoreResult<ObservedIntakePins> {
    Ok(ObservedIntakePins {
        mint: object(proof.mint_bytes())?,
        receipt: object(proof.receipt_bytes())?,
    })
}

fn object(raw: &[u8]) -> StoreResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&super::super::codec::digest(raw))
            .map_err(super::super::source_error)?,
        bytes: u64::try_from(raw.len()).map_err(|_| Error::BudgetExceeded)?,
    })
}

fn packet(
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PublicSchoolIntakePacketV1> {
    let held = evidence.retained_root(grant)?;
    let raw = held.payload().get().as_bytes();
    let cost = raw.len().checked_mul(4).ok_or(Error::BudgetExceeded)?;
    grant
        .charge_bytes(cost)
        .map_err(super::super::source_error)?;
    grant.charge_steps(64).map_err(super::super::source_error)?;
    let packet = PublicSchoolIntakePacketV1::decode(raw)?;
    if packet.reference().as_str() != held.root().key
        || packet.directory().work().slot().store_identity() != evidence.store_identity()
    {
        return Err(Error::Corrupt("owned intake packet lineage").into());
    }
    Ok(packet)
}

pub(in crate::roster_source_issuance) fn inspect(
    view: &StoreSnapshot<'_>,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<PublicSchoolIntakeAdmissionV1>> {
    if evidence.retained_root(grant)?.root().schema != Schema::ProviderSchoolObservedPacketV1 {
        return Ok(None);
    }
    let packet = packet(evidence, grant)?;
    grant
        .charge_bytes(2 * 65_536)
        .map_err(super::super::source_error)?;
    grant.charge_steps(8).map_err(super::super::source_error)?;
    view.read_public_school_intake_admission_v1(&packet, &mut AcquisitionReadBudget::for_work())
        .map(Some)
}

pub(in crate::roster_source_issuance) fn restore(
    evidence: &RosterSourceEvidenceV1,
    expected: Option<&ObservedIntakePins>,
    objects: &BTreeMap<String, Vec<u8>>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<PublicSchoolIntakeAdmissionV1>> {
    let Some(expected) = expected else {
        if evidence.retained_root(grant)?.root().schema == Schema::ProviderSchoolObservedPacketV1 {
            return Err(Error::Corrupt("owned intake admission absent").into());
        }
        return Ok(None);
    };
    let packet = packet(evidence, grant)?;
    let mint = retained_object(objects, &expected.mint, grant)?;
    let receipt = retained_object(objects, &expected.receipt, grant)?;
    PublicSchoolIntakeAdmissionV1::from_retained(&packet, mint, receipt).map(Some)
}

fn retained_object<'a>(
    objects: &'a BTreeMap<String, Vec<u8>>,
    reference: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a [u8]> {
    let raw = objects
        .get(reference.digest.as_str())
        .ok_or(Error::Corrupt("owned intake proof object absent"))?;
    if raw.is_empty() || raw.len() > 65_536 {
        return Err(Error::BudgetExceeded.into());
    }
    grant
        .charge_bytes(raw.len().checked_mul(3).ok_or(Error::BudgetExceeded)?)
        .map_err(super::super::source_error)?;
    grant.charge_steps(4).map_err(super::super::source_error)?;
    if object(raw)? != *reference {
        return Err(Error::Corrupt("owned intake proof object differs").into());
    }
    Ok(raw)
}
