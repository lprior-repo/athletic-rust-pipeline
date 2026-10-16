use super::{admission, affected, claims, context, dispositions, members};
use crate::clock::{Clock, SystemClock};
use crate::journal::context::ContextCharge;
use crate::public_identity::{qualification::Ledger, PublicIdentityError};
use crate::{Store, StoreResult, StoreSnapshot};
use census_domain::model::{AthleteCandidateId, StoreIdentity};
use census_domain::public_identity_history::local_adjudication::RetainedClaimV1;
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV1,
    LOCAL_MEMBERSHIP_WITHHOLDING_POLICY,
};
use census_domain::public_identity_history::RegistryHead;

pub(super) struct Captured {
    pub(super) inputs: context::Inputs,
    pub(super) affected: affected::Inputs,
    pub(super) members: Vec<AthleteCandidateId>,
    pub(super) inventory: Vec<RetainedClaimV1>,
    pub(super) charges: [ContextCharge; 2],
}

pub(super) fn capture(
    store: &Store,
    snapshot: &StoreSnapshot<'_>,
    identity: &StoreIdentity,
    request: &LocalMembershipWithholdV1,
    ledger: &Ledger,
    reserved: Option<&admission::Reservation>,
) -> StoreResult<Captured> {
    let inputs = context::Inputs::capture(snapshot)?;
    let members = members(&inputs, ledger.history.projection(), &request.canonical)?;
    let inventory = claims::membership_inventory(
        &inputs,
        &members,
        ledger.history.projection(),
        ledger.history.records(),
    )?;
    let authority_context = inputs.measure_context(identity)?;
    let mut affected = affected::Inputs::capture(snapshot, identity, &authority_context)?;
    if let Some(reserved) = reserved {
        reserved.restore_affected(&mut affected)?;
    }
    if affected.record_keys(&inputs, &members)? != request.affected_record_keys {
        return Err(
            PublicIdentityError::Corrupt("membership withholding affected inventory").into(),
        );
    }
    let charges = [
        inputs.admission_charge(store, identity)?,
        affected.admission_charge(store)?,
    ];
    Ok(Captured {
        inputs,
        affected,
        members,
        inventory,
        charges,
    })
}

impl Captured {
    pub(super) fn fresh_record(
        &self,
        identity: &StoreIdentity,
        operation: &str,
        request: LocalMembershipWithholdV1,
        head: &RegistryHead,
        snapshot: &StoreSnapshot<'_>,
    ) -> StoreResult<LocalMembershipWithholdingRecordV1> {
        let [authority, affected] = &self.charges;
        Ok(LocalMembershipWithholdingRecordV1 {
            version: 1,
            policy: LOCAL_MEMBERSHIP_WITHHOLDING_POLICY.into(),
            store: identity.clone(),
            operation: operation.into(),
            expected_head: head.clone(),
            evidence_generation: snapshot.evidence_generation(),
            derived_generation: snapshot.derived_generation(),
            snapshot_sequence: snapshot.sequence(),
            authority_context: authority.root.clone(),
            affected_context: affected.root.clone(),
            members: self.members.clone(),
            dispositions: dispositions(&request, &self.inventory)?,
            request,
            observed_at: SystemClock.today_iso8601(),
        })
    }
}
