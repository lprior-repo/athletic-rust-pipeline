use crate::public_identity::{
    authority, codec, context,
    membership::SourceCandidateMembershipWitnessV1,
    operation::HeldWitness,
    retained::{Live, Read, Rows},
    witness, PublicIdentityError,
};
use crate::{Store, StoreResult};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1;
use census_domain::public_identity_history::{HistoryRecord, HistoryRequest};

pub(super) fn validate_live(
    store: &Store,
    record: &LocalMembershipWithholdingRecordV1,
    records: &[HistoryRecord],
) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    validate(
        &Live {
            snapshot: &snapshot,
            entities: &store.entities,
            meta: &store.meta,
            journal: &store.journal,
            receipts: &store.receipts,
        },
        record,
        records,
    )
    .map(|_| ())
}

pub(super) fn validate(
    reader: &impl Read,
    record: &LocalMembershipWithholdingRecordV1,
    records: &[HistoryRecord],
) -> StoreResult<SourceCandidateMembershipWitnessV1> {
    let prior = &record.request.attachment;
    if prior.sequence > record.expected_head.sequence {
        return Err(PublicIdentityError::Corrupt("withholding future attachment").into());
    }
    let original = records
        .iter()
        .find(|entry| entry.sequence == prior.sequence)
        .ok_or(PublicIdentityError::Corrupt(
            "withholding attachment missing",
        ))?;
    let HistoryRequest::BindCandidateMembership { membership, .. } = &original.request else {
        return Err(PublicIdentityError::Corrupt("withholding attachment kind").into());
    };
    if original.digest != prior.digest || membership.id != prior.membership_id {
        return Err(PublicIdentityError::Corrupt("withholding attachment reference").into());
    }
    let key = witness::key(prior.sequence);
    let raw = reader
        .row(Rows::Witness, &key)?
        .ok_or(PublicIdentityError::Corrupt(
            "withholding attachment witness missing",
        ))?;
    let HeldWitness::Membership(held) = HeldWitness::decode(&key, &raw)? else {
        return Err(PublicIdentityError::Corrupt("withholding attachment witness kind").into());
    };
    if held.record_digest != prior.digest || held.request.case_id != record.request.case_id {
        return Err(PublicIdentityError::Corrupt("withholding attachment case binding").into());
    }
    Ok(held)
}

pub(in crate::public_identity) fn material(
    reader: &impl Read,
    record: &LocalMembershipWithholdingRecordV1,
) -> StoreResult<String> {
    let key = witness::key(record.request.attachment.sequence);
    let raw = reader
        .row(Rows::Witness, &key)?
        .ok_or(PublicIdentityError::Corrupt("missing withheld attachment"))?;
    let HeldWitness::Membership(held) = HeldWitness::decode(&key, &raw)? else {
        return Err(PublicIdentityError::Corrupt("withheld attachment kind").into());
    };
    attachment_material(reader, &record.store, &held)
}

pub(in crate::public_identity) fn attachment_material(
    reader: &impl Read,
    identity: &StoreIdentity,
    held: &SourceCandidateMembershipWitnessV1,
) -> StoreResult<String> {
    let inputs = reader.context(&held.authority_context, identity)?;
    let case = inputs
        .cases
        .iter()
        .find(|case| case.id == held.request.case_id)
        .ok_or(PublicIdentityError::NotReviewed)?;
    authority::identity_material(&inputs, &case.member_ids)
}

pub(in crate::public_identity) fn require_available(
    store: &Store,
    records: &[HistoryRecord],
    inputs: &context::Inputs,
    case_id: &str,
) -> StoreResult<()> {
    let case = inputs
        .cases
        .iter()
        .find(|case| case.id == case_id)
        .ok_or(PublicIdentityError::NotReviewed)?;
    let current = authority::identity_material(inputs, &case.member_ids)?;
    let snapshot = store.db.snapshot();
    let reader = Live {
        snapshot: &snapshot,
        entities: &store.entities,
        meta: &store.meta,
        journal: &store.journal,
        receipts: &store.receipts,
    };
    for record in records {
        let key = witness::key(record.sequence);
        let raw = codec::load(&snapshot, &store.journal, &key, 1024 * 1024)?
            .ok_or(PublicIdentityError::Corrupt("missing withholding witness"))?;
        if let HeldWitness::MembershipWithheld(held) = HeldWitness::decode(&key, &raw)? {
            if material(&reader, &held.adjudication)? == current {
                return Err(
                    PublicIdentityError::Corrupt("membership authority locally withheld").into(),
                );
            }
        }
    }
    Ok(())
}
