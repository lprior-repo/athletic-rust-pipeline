use crate::public_identity::{
    operation::HeldWitness,
    retained::{Read, Rows},
    witness, PublicIdentityError,
};
use crate::{Store, StoreResult};
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV1;
use census_domain::public_identity_history::HistoryRecord;

pub(in crate::public_identity) fn require_available(
    store: &Store,
    records: &[HistoryRecord],
    inputs: &crate::public_identity::context::Inputs,
    case_id: &str,
) -> StoreResult<()> {
    let case = inputs
        .cases
        .iter()
        .find(|case| case.id == case_id)
        .ok_or(PublicIdentityError::NotReviewed)?;
    let material = crate::public_identity::authority::identity_material(inputs, &case.member_ids)?;
    let snapshot = store.db.snapshot();
    for record in records {
        let key = witness::key(record.sequence);
        let raw = crate::public_identity::codec::load(
            &snapshot,
            &store.journal,
            &key,
            super::MAX_WITNESS_BYTES,
        )?
        .ok_or(PublicIdentityError::Corrupt("missing commit witness"))?;
        if let HeldWitness::Manual(held) = HeldWitness::decode(&key, &raw)? {
            let prior = crate::public_identity::context::read_inputs(
                &snapshot,
                &store.journal,
                &held.adjudication.authority_context,
                &held.adjudication.store,
            )?;
            let prior_material = crate::public_identity::authority::identity_material(
                &prior,
                &held.adjudication.members,
            )?;
            if prior_material == material {
                return Err(PublicIdentityError::Corrupt(
                    "reviewed authority superseded by local adjudication",
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn validate_superseded(
    reader: &impl Read,
    record: &LocalIdentityAdjudicationRecordV1,
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
        let HeldWitness::Reviewed(held) = HeldWitness::decode(&key, &raw)? else {
            return Err(PublicIdentityError::Corrupt("manual superseded authority kind").into());
        };
        if held.case_id != record.request.case_id || held.record_digest != prior.digest {
            return Err(PublicIdentityError::Corrupt("manual superseded case binding").into());
        }
    }
    Ok(())
}

pub(super) fn validate_live(
    store: &Store,
    record: &LocalIdentityAdjudicationRecordV1,
) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    validate_superseded(
        &crate::public_identity::retained::Live {
            snapshot: &snapshot,
            entities: &store.entities,
            meta: &store.meta,
            journal: &store.journal,
            receipts: &store.receipts,
        },
        record,
    )
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
    record: &LocalIdentityAdjudicationRecordV1,
) -> StoreResult<String> {
    let inputs = reader.context(&record.authority_context, &record.store)?;
    crate::public_identity::authority::identity_material(&inputs, &record.members)
}
