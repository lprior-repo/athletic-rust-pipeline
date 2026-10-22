use super::*;
use crate::public_identity::graph_reference::GraphAuthorityReferenceV1;
use fjall::{Keyspace, Readable, Snapshot};
use std::collections::BTreeMap;

mod receipts;

struct Group {
    receipt: crate::Receipt,
    graph: GraphAuthorityReferenceV1,
    candidates: BTreeSet<AthleteCandidateId>,
    previous: Option<(u64, AthleteCandidateId)>,
    count: u64,
}

pub(in crate::public_identity) fn audit(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    identity: &StoreIdentity,
    records: &[HistoryRecord],
) -> StoreResult<()> {
    let mut groups = receipts::groups(snapshot, journal, receipts, identity, records.len())?;
    for record in records {
        let key = witness::key(record.sequence);
        let raw = codec::load(
            snapshot,
            journal,
            &key,
            crate::public_identity::local_adjudication::MAX_WITNESS_BYTES,
        )?
        .ok_or(PublicIdentityError::Corrupt("missing cohort audit witness"))?;
        let held = HeldGraphWitness::decode(&key, &raw)?;
        if is_reserved_operation(held.operation().as_bytes()) {
            accept(&mut groups, identity, record, &held)?;
        }
    }
    if groups
        .values()
        .any(|group| group.count != group.receipt.appended)
    {
        return Err(PublicIdentityError::Corrupt("cohort completion count").into());
    }
    Ok(())
}

fn accept(
    groups: &mut BTreeMap<String, Group>,
    identity: &StoreIdentity,
    record: &HistoryRecord,
    held: &HeldGraphWitness,
) -> StoreResult<()> {
    let suffix = held
        .operation()
        .strip_prefix(PREFIX)
        .ok_or(PublicIdentityError::Corrupt("cohort child prefix"))?;
    let (digest, _) = suffix
        .split_once(':')
        .ok_or(PublicIdentityError::Corrupt("cohort child grammar"))?;
    let group = groups.get_mut(digest).ok_or(PublicIdentityError::Corrupt(
        "cohort children without completion receipt",
    ))?;
    let Payload::Source { candidate, .. } = &held.payload else {
        return Err(PublicIdentityError::Corrupt("cohort child payload").into());
    };
    let expected = format!("{PREFIX}{digest}:{}", serialized_digest(candidate)?);
    if held.operation() != expected
        || held.request_digest() != witness::request_digest(identity, &expected, candidate)?
        || held.record_digest != record.digest
        || held.observed_at() != group.receipt.at
        || held.graph() != &group.graph
        || !group.candidates.insert(candidate.clone())
    {
        return Err(PublicIdentityError::Corrupt("cohort child binding").into());
    }
    if group.previous.as_ref().is_some_and(|(sequence, previous)| {
        sequence.checked_add(1) != Some(record.sequence) || previous >= candidate
    }) {
        return Err(PublicIdentityError::Corrupt("cohort child order").into());
    }
    group.previous = Some((record.sequence, candidate.clone()));
    group.count = group
        .count
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    if group.count > group.receipt.appended {
        return Err(PublicIdentityError::Corrupt("cohort child count").into());
    }
    Ok(())
}
