use super::{
    codec, correction, local_adjudication, qualification::Ledger, witness, IdentityCommitOutcomeV1,
    PublicIdentityError,
};
use crate::{Store, StoreResult};
use serde::Deserialize;

pub(super) enum HeldWitness {
    Source(witness::Witness),
    Reviewed(correction::ReviewedWitness),
    Manual(local_adjudication::LocalAdjudicationWitnessV1),
}

impl HeldWitness {
    pub(super) fn decode(key: &str, bytes: &[u8]) -> StoreResult<Self> {
        #[derive(Deserialize)]
        struct Version {
            version: u32,
        }
        if bytes.len() > local_adjudication::MAX_WITNESS_BYTES {
            return Err(crate::StoreError::RowTooLarge {
                bytes: bytes.len(),
                max: local_adjudication::MAX_WITNESS_BYTES,
            });
        }
        let version: StoreResult<Version> = codec::decode(key, bytes);
        if bytes.len() > 65_536 && !matches!(&version, Ok(Version { version: 4 })) {
            return Err(crate::StoreError::RowTooLarge {
                bytes: bytes.len(),
                max: 65_536,
            });
        }
        let version = version?;
        match version.version {
            1 | 2 => codec::decode(key, bytes).map(Self::Source),
            3 => codec::decode(key, bytes).map(Self::Reviewed),
            4 => codec::decode(key, bytes).map(Self::Manual),
            _ => Err(PublicIdentityError::Corrupt("unsupported identity witness version").into()),
        }
    }
    pub(super) fn operation(&self) -> &str {
        match self {
            Self::Source(held) => &held.operation,
            Self::Reviewed(held) => &held.operation,
            Self::Manual(held) => &held.operation,
        }
    }
    pub(super) fn decision_id(&self) -> Option<&str> {
        match self {
            Self::Source(held) => Some(&held.decision_id),
            Self::Reviewed(held) => Some(&held.decision_id),
            Self::Manual(_) => None,
        }
    }
    pub(super) fn authority_context(&self) -> Option<&str> {
        match self {
            Self::Source(held) => held.authority_context.as_deref(),
            Self::Reviewed(held) => Some(&held.authority_context),
            Self::Manual(held) => Some(&held.adjudication.authority_context),
        }
    }
    fn request_digest(&self) -> &str {
        match self {
            Self::Source(held) => &held.request_digest,
            Self::Reviewed(held) => &held.request_digest,
            Self::Manual(held) => &held.request_digest,
        }
    }
}

pub(super) enum Replayed {
    Source(IdentityCommitOutcomeV1),
    Reviewed(correction::ReviewedIdentityCommitOutcomeV1),
    Manual(local_adjudication::LocalAdjudicationCommitOutcomeV1),
}

pub(super) fn repeated(
    store: &Store,
    operation: &str,
    digest: &str,
    ledger: &Ledger,
) -> StoreResult<Option<Replayed>> {
    let snapshot = store.db.snapshot();
    let key = witness::receipt_key(operation);
    let Some(raw) = codec::load(&snapshot, &store.receipts, &key, 65_536)? else {
        return Ok(None);
    };
    let receipt: crate::Receipt = codec::decode(&key, &raw)?;
    if receipt.operation != key {
        return Err(PublicIdentityError::Corrupt("receipt operation").into());
    }
    for record in ledger.history.records() {
        let key = witness::key(record.sequence);
        let raw = codec::load(
            &snapshot,
            &store.journal,
            &key,
            local_adjudication::MAX_WITNESS_BYTES,
        )?
        .ok_or(PublicIdentityError::Corrupt("missing replay witness"))?;
        let held = HeldWitness::decode(&key, &raw)?;
        if held.operation() == operation {
            if held.request_digest() != digest {
                return Err(PublicIdentityError::OperationConflict.into());
            }
            return match held {
                HeldWitness::Source(held) => {
                    held.outcome(record, true).map(Replayed::Source).map(Some)
                }
                HeldWitness::Reviewed(held) => {
                    held.outcome(record, true).map(Replayed::Reviewed).map(Some)
                }
                HeldWitness::Manual(held) => {
                    held.outcome(record, true).map(Replayed::Manual).map(Some)
                }
            };
        }
    }
    Err(PublicIdentityError::Corrupt("receipt without history witness").into())
}

pub(super) fn accepted_decisions(
    snapshot: &fjall::Snapshot,
    journal: &fjall::Keyspace,
    records: &[census_domain::public_identity_history::HistoryRecord],
) -> StoreResult<usize> {
    records.iter().try_fold(0usize, |count, record| {
        let key = witness::key(record.sequence);
        let raw = codec::load(
            snapshot,
            journal,
            &key,
            local_adjudication::MAX_WITNESS_BYTES,
        )?
        .ok_or(PublicIdentityError::Corrupt("missing inventory witness"))?;
        if HeldWitness::decode(&key, &raw)?.decision_id().is_some() {
            count
                .checked_add(1)
                .ok_or(crate::StoreError::CounterOverflow)
        } else {
            Ok(count)
        }
    })
}
