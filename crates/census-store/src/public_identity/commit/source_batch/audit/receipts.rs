use super::*;
use crate::public_identity::local_adjudication::admission;
use crate::public_identity::paged_generation::RawGenerationOperationIdV2;
use serde::Deserialize;

#[derive(Deserialize)]
struct ReservationOwner {
    version: u8,
    purpose: Purpose,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Purpose {
    RawGeneration {
        operation: RawGenerationOperationIdV2,
    },
}

pub(super) fn groups(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    identity: &StoreIdentity,
    maximum: usize,
) -> StoreResult<BTreeMap<String, Group>> {
    let mut groups = BTreeMap::new();
    for guard in snapshot.prefix(receipts, PREFIX) {
        if groups.len() >= maximum {
            return Err(PublicIdentityError::Corrupt("cohort completion cardinality").into());
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let key = std::str::from_utf8(&key)
            .map_err(|_| PublicIdentityError::Corrupt("cohort completion key encoding"))?;
        let suffix = key
            .strip_prefix(PREFIX)
            .ok_or(PublicIdentityError::Corrupt("cohort completion prefix"))?;
        if !crate::journal::is_object_id(suffix) {
            return Err(PublicIdentityError::Corrupt("cohort completion key").into());
        }
        let raw = codec::load(snapshot, receipts, key, 65_536)?.ok_or(
            PublicIdentityError::Corrupt("cohort completion disappeared"),
        )?;
        let receipt: crate::Receipt = codec::decode(key, &raw)?;
        require_receipt(&receipt, key, maximum)?;
        let graph = reserved_graph(snapshot, journal, identity, key)?;
        let digest = receipt.digest.clone();
        let group = Group {
            receipt,
            graph,
            candidates: BTreeSet::new(),
            previous: None,
            count: 0,
        };
        if groups.insert(digest, group).is_some() {
            return Err(PublicIdentityError::Corrupt("duplicate cohort request digest").into());
        }
    }
    Ok(groups)
}

fn require_receipt(receipt: &crate::Receipt, key: &str, maximum: usize) -> StoreResult<()> {
    let maximum = u64::try_from(maximum).map_err(|_| StoreError::CounterOverflow)?;
    if receipt.operation != key
        || !crate::journal::is_object_id(&receipt.digest)
        || receipt.appended == 0
        || receipt.appended > maximum
        || receipt.credited
        || receipt.at.is_empty()
        || receipt.at.len() > 256
        || receipt.at.chars().any(char::is_control)
    {
        return Err(PublicIdentityError::Corrupt("cohort completion receipt").into());
    }
    Ok(())
}

fn reserved_graph(
    snapshot: &Snapshot,
    journal: &Keyspace,
    identity: &StoreIdentity,
    completion: &str,
) -> StoreResult<GraphAuthorityReferenceV1> {
    let operation = graph_operation::raw_operation(identity, completion)?;
    let key = admission::key(operation.as_str());
    let raw = codec::load(
        snapshot,
        journal,
        &key,
        admission::graph::MAX_METADATA_BYTES,
    )?
    .ok_or(PublicIdentityError::Corrupt("cohort raw reservation"))?;
    let reservation: admission::graph::GraphReservation = codec::decode(&key, &raw)?;
    let owner: ReservationOwner = codec::decode(&key, &raw)?;
    let Purpose::RawGeneration {
        operation: retained,
    } = owner.purpose;
    if owner.version != 3
        || retained != operation
        || reservation.capture().store_identity() != identity
    {
        return Err(PublicIdentityError::Corrupt("cohort reservation owner").into());
    }
    Ok(GraphAuthorityReferenceV1::new(
        reservation.capture().clone(),
        reservation.seal().clone(),
    ))
}
