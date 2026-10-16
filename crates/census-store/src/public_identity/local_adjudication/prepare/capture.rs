use super::{admission, affected, claims, context, members, validate_case};
use crate::clock::{Clock, SystemClock};
use crate::journal::context::ContextCharge;
use crate::public_identity::{PublicIdentityError, PublicIdentityWriterV1};
use crate::{StoreResult, StoreSnapshot};
use census_domain::model::{AthleteCandidateId, StoreIdentity};
use census_domain::public_identity_history::local_adjudication::{
    LocalEvidenceBasisV1, LocalIdentityAdjudicationRecordV2, LocalIdentitySplitV1, RetainedClaimV1,
    LOCAL_ADJUDICATION_POLICY,
};
use census_domain::public_identity_history::{HistoryProjection, RegistryHead};

pub(super) struct Captured {
    pub(super) inputs: context::Inputs,
    pub(super) affected: affected::Inputs,
    pub(super) members: Vec<AthleteCandidateId>,
    pub(super) claims: Vec<RetainedClaimV1>,
}

pub(super) fn capture(
    snapshot: &StoreSnapshot<'_>,
    identity: &StoreIdentity,
    request: &LocalIdentitySplitV1,
    projection: &HistoryProjection,
    records: &[census_domain::public_identity_history::HistoryRecord],
    reserved: Option<&admission::Reservation>,
) -> StoreResult<Captured> {
    let inputs = context::Inputs::capture(snapshot)?;
    let members = members(&inputs, projection, &request.canonical)?;
    validate_case(&inputs, request, &members)?;
    let claims = claims::membership_inventory(&inputs, &members, projection, records)?;
    let authority = inputs.measure_context(identity)?;
    let mut affected = affected::Inputs::capture(snapshot, identity, &authority)?;
    if let Some(reserved) = reserved {
        reserved.restore_affected(&mut affected)?;
    }
    if affected.record_keys(&inputs, &members)? != request.affected_record_keys {
        return Err(PublicIdentityError::Corrupt("manual affected record inventory").into());
    }
    Ok(Captured {
        inputs,
        affected,
        members,
        claims,
    })
}

pub(super) fn fresh_record(
    writer: &PublicIdentityWriterV1,
    operation: &str,
    request: LocalIdentitySplitV1,
    head: &RegistryHead,
    snapshot: &StoreSnapshot<'_>,
    captured: &Captured,
    charges: &[ContextCharge; 2],
) -> LocalIdentityAdjudicationRecordV2 {
    let [authority, affected] = charges;
    LocalIdentityAdjudicationRecordV2 {
        version: 2,
        policy: LOCAL_ADJUDICATION_POLICY.into(),
        store: writer.identity.clone(),
        operation: operation.into(),
        expected_head: head.clone(),
        evidence_generation: snapshot.evidence_generation(),
        derived_generation: snapshot.derived_generation(),
        snapshot_sequence: snapshot.sequence(),
        authority_context: authority.root.clone(),
        affected_context: affected.root.clone(),
        members: captured.members.clone(),
        request,
        observed_at: SystemClock.today_iso8601(),
        evidence_basis: LocalEvidenceBasisV1::RetainedObservationOnly,
    }
}
