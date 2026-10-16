use crate::public_identity::{
    operation::HeldWitness,
    retained::{HistoricalRead as Read, Rows},
    witness, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;

pub(super) fn validate_superseded(
    reader: &impl Read,
    record: &LocalIdentityAdjudicationRecordV2,
) -> StoreResult<()> {
    for prior in &record.request.supersedes {
        if prior.sequence > record.expected_head.sequence {
            return Err(
                PublicIdentityError::Corrupt("manual superseded operation position").into(),
            );
        }
        let key = witness::key(prior.sequence);
        let raw = reader
            .row(Rows::Witness, &key)?
            .ok_or(PublicIdentityError::Corrupt(
                "missing superseded reviewed witness",
            ))?;
        let (case_id, record_digest) = match HeldWitness::decode(&key, &raw)? {
            HeldWitness::Reviewed(held) => (held.case_id, held.record_digest),
            HeldWitness::Membership(held) => (held.request.case_id, held.record_digest),
            _ => {
                return Err(PublicIdentityError::Corrupt("manual superseded authority kind").into())
            }
        };
        if case_id != record.request.case_id || record_digest != prior.digest {
            return Err(PublicIdentityError::Corrupt("manual superseded case binding").into());
        }
    }
    Ok(())
}

pub(in crate::public_identity) fn reviewed_material(
    reader: &impl Read,
    identity: &census_domain::model::StoreIdentity,
    held: &crate::public_identity::correction::ReviewedWitness,
) -> StoreResult<String> {
    let inputs = reader.context(&held.authority_context, identity)?;
    let case = inputs
        .cases
        .iter()
        .find(|case| case.id == held.case_id)
        .ok_or(PublicIdentityError::Corrupt("reviewed case missing"))?;
    crate::public_identity::authority::identity_material(&inputs, &case.member_ids)
}

pub(in crate::public_identity) fn manual_material(
    reader: &impl Read,
    record: &LocalIdentityAdjudicationRecordV2,
) -> StoreResult<String> {
    let inputs = reader.context(&record.authority_context, &record.store)?;
    crate::public_identity::authority::identity_material(&inputs, &record.members)
}
