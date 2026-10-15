use super::{request::ReviewedKind, witness, ReviewedWitness};
use crate::public_identity::{codec, context, witness as source_witness, PublicIdentityError};
use crate::StoreResult;
use census_domain::model::{
    serialized_digest, AppliedAthleteIdentity, AppliedIdentityKind, StoreIdentity,
};
use census_domain::public_identity_history::{HistoryRecord, ReplayLimits};

pub(super) fn verify(
    witness: &ReviewedWitness,
    reader: &impl crate::public_identity::retained::Read,
    identity: &StoreIdentity,
    record: &HistoryRecord,
) -> StoreResult<()> {
    let key = source_witness::receipt_key(&witness.operation);
    let raw = reader
        .row(crate::public_identity::retained::Rows::Receipt, &key)?
        .ok_or(PublicIdentityError::Corrupt("missing reviewed receipt"))?;
    let receipt: crate::Receipt = codec::decode(&key, &raw)?;
    if receipt.operation != key
        || receipt.digest != serialized_digest(witness)?
        || receipt.appended != 1
        || receipt.credited
    {
        return Err(PublicIdentityError::Corrupt("reviewed receipt binding").into());
    }
    let raw = reader
        .row(
            crate::public_identity::retained::Rows::Decision,
            &witness.decision_id,
        )?
        .ok_or(PublicIdentityError::Corrupt("missing reviewed decision"))?;
    let decision: AppliedAthleteIdentity = codec::decode(&witness.decision_id, &raw)?;
    let evidence = witness::evidence(&record.request)?;
    let matches = match (witness.kind, decision.kind) {
        (ReviewedKind::Merge, AppliedIdentityKind::SamePerson) => {
            evidence.matches_applied(&decision, ReplayLimits::SMALL)
        }
        (ReviewedKind::Split | ReviewedKind::Revoke, AppliedIdentityKind::DifferentPerson) => {
            evidence.matches_correction(&decision, ReplayLimits::SMALL)
        }
        _ => return Err(PublicIdentityError::Corrupt("reviewed decision kind").into()),
    }
    .map_err(PublicIdentityError::from)?;
    if !matches
        || receipt.at != decision.observed_at
        || decision.id != witness.decision_id
        || decision.case_id.as_deref() != Some(witness.case_id.as_str())
        || serialized_digest(&decision)? != witness.decision_digest
    {
        return Err(PublicIdentityError::Corrupt("reviewed decision binding").into());
    }
    let inputs = reader.context(&witness.authority_context, identity)?;
    context::verify_reviewed_inputs(&inputs, &decision)
}
