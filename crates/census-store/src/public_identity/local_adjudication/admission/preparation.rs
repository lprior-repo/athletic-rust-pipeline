use super::{
    add, codec, header, header_payload_limit, invalid, key, require_capacity, Store, StoreError,
    StoreIdentity, StoreResult, HEADER, MAX_JOURNAL_VALUE_BYTES,
};
use crate::public_identity::{
    graph_reference::GraphAuthorityReferenceV1, graph_witness::HeldGraphWitness,
    paged_generation::RawGenerationOperationIdV2,
};
use census_domain::model::serialized_digest;
use census_domain::public_identity_history::RegistryHead;
use fjall::PersistMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct Preparation {
    pub version: u8,
    pub store: StoreIdentity,
    pub operation: String,
    pub request_digest: String,
    #[serde(deserialize_with = "crate::public_identity::graph_witness::decode_head")]
    pub expected_head: RegistryHead,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub raw_operation: RawGenerationOperationIdV2,
    pub graph: GraphAuthorityReferenceV1,
    pub observed_at: String,
    pub witness: HeldGraphWitness,
}
impl Preparation {
    pub(in crate::public_identity) fn require_request(
        &self,
        identity: &StoreIdentity,
        operation: &str,
        digest: &str,
        expected: crate::StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<()> {
        if &self.store != identity
            || self.operation != operation
            || self.request_digest != digest
            || &self.expected_head != head
            || self.evidence_generation != expected.evidence_generation()
            || self.derived_generation != expected.derived_generation()
        {
            return Err(crate::PublicIdentityError::OperationConflict.into());
        }
        Ok(())
    }
    fn validate(&self, identity: &StoreIdentity) -> StoreResult<()> {
        self.graph.validate()?;
        if self.version != 4
            || &self.store != identity
            || self.operation != self.witness.operation()
            || self.graph != *self.witness.graph()
            || self.observed_at != self.witness.observed_at()
            || self.raw_operation
                != crate::public_identity::graph_operation::raw_operation(
                    identity,
                    &self.operation,
                )?
            || !crate::journal::is_object_id(&self.request_digest)
            || &self.store != self.graph.capture().store_identity()
            || self.evidence_generation != self.graph.capture().evidence_generation()
            || self.derived_generation != self.graph.capture().derived_generation()
        {
            return Err(invalid());
        }
        self.witness.validate_shape()?;
        Ok(())
    }
}

pub(in crate::public_identity) fn load(
    store: &Store,
    identity: &StoreIdentity,
    operation: &str,
) -> StoreResult<Option<Preparation>> {
    header(store, identity)?;
    let key = key(operation);
    let Some(raw) = codec::load(
        &store.db.snapshot(),
        &store.journal,
        &key,
        MAX_JOURNAL_VALUE_BYTES,
    )?
    else {
        return Ok(None);
    };
    let preparation: Preparation = codec::decode(&key, &raw)?;
    preparation.validate(identity)?;
    if preparation.operation != operation {
        return Err(invalid());
    }
    Ok(Some(preparation))
}

pub(in crate::public_identity) fn reserve(
    store: &Store,
    preparation: Preparation,
) -> StoreResult<Preparation> {
    preparation.validate(&preparation.store)?;
    if let Some(prior) = load(store, &preparation.store, &preparation.operation)? {
        if serialized_digest(&prior)? != serialized_digest(&preparation)? {
            return Err(invalid());
        }
        return Ok(prior);
    }
    let mut header = header(store, &preparation.store)?;
    let key = key(&preparation.operation);
    let limit = MAX_JOURNAL_VALUE_BYTES
        .checked_sub(key.len())
        .ok_or(StoreError::CounterOverflow)?;
    let raw = codec::encode(&preparation, limit)?;
    #[cfg(feature = "native-fault-injection")]
    let input_digest = crate::journal::object_digest(&raw);
    header.bytes = add(header.bytes, metadata_charge(key.len(), raw.len())?)?;
    header.reservations = add(header.reservations, 1)?;
    require_capacity(header.bytes, header.roots, header.reservations)?;
    let encoded = codec::encode(&header, header_payload_limit()?)?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key.as_bytes(), raw);
    batch.insert(&store.journal, HEADER.as_bytes(), encoded);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    #[cfg(feature = "native-fault-injection")]
    crate::native_worker_boundary::pause(
        crate::native_worker_boundary::Position::PublicIdentityPreparationCommittedBeforeReturn {
            sequence: preparation.expected_head.sequence,
        },
        &preparation.operation,
        &input_digest,
    )?;
    Ok(preparation)
}

pub(super) fn audit(
    store: &Store,
    row_key: &str,
    raw: &[u8],
    identity: &StoreIdentity,
) -> StoreResult<(u64, u64)> {
    let metadata_bytes = raw.len();
    let preparation: Preparation = codec::decode(row_key, raw)?;
    preparation.validate(identity)?;
    if row_key != key(&preparation.operation) {
        return Err(invalid());
    }
    let graph_key = key(preparation.raw_operation.as_str());
    let raw = codec::load(
        &store.db.snapshot(),
        &store.journal,
        &graph_key,
        super::graph_wire::MAX_METADATA_BYTES,
    )?
    .ok_or_else(invalid)?;
    let graph: super::graph::GraphReservation = codec::decode(&graph_key, &raw)?;
    if graph.capture() != preparation.graph.capture()
        || graph.seal() != preparation.graph.seal()
        || !graph.is_complete(store)?
    {
        return Err(invalid());
    }
    Ok((metadata_charge(row_key.len(), metadata_bytes)?, 0))
}
fn metadata_charge(key: usize, value: usize) -> StoreResult<u64> {
    let bytes = key.checked_add(value).ok_or(StoreError::CounterOverflow)?;
    if bytes > MAX_JOURNAL_VALUE_BYTES {
        return Err(invalid());
    }
    u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)
}
