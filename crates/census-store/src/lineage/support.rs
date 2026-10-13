use census_domain::model::{StoreIdentity, StoreIdentityError};
use fjall::{Keyspace, PersistMode, Readable, Snapshot};
use std::fmt::Debug;

use crate::{Store, StoreError, StoreFormat, StoreLineageError, StoreResult, Table};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(super) const PHASE: &str = "export-lineage-v1";
pub(super) const KEY: &str = "store-identity";
pub(super) const LOWER: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub(super) const UPPER: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
pub(super) const MIXED: &str = "0123456789aBcDeF0123456789aBcDeF0123456789aBcDeF0123456789aBcDeF";
pub(super) const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";
pub(super) const SCHOOL_DIGEST: &str =
    "598e378cbee0e81899daab1d45650bee84389e909423a0cbd1f9a9982fb9cca0";
pub(super) const OLD_UPPER_ENVELOPE: &[u8] = br#"{"at":"2020-01-02T03:04:05Z","key":"store-identity","payload":"0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"}"#;
pub(super) const FIXTURE_ROOT: &str = "CENSUS_LINEAGE_FIXTURE_ROOT";

pub(super) fn fixture_dir() -> TestResult<tempfile::TempDir> {
    match std::env::var_os(FIXTURE_ROOT) {
        Some(root) => Ok(tempfile::Builder::new()
            .prefix("lineage-")
            .tempdir_in(root)?),
        None => Ok(tempfile::tempdir()?),
    }
}

pub(super) fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

pub(super) fn seed(store: &Store) -> TestResult {
    store.append(
        Table::Schools,
        &serde_json::json!({"id":"lineage-school","note":"synthetic-evidence"}),
    )?;
    equal(store.evidence_generation(), 1)?;
    equal(store.derived_generation(), 0)?;
    equal(
        store.snapshot().tables_digest(&[Table::Schools])?.as_str(),
        SCHOOL_DIGEST,
    )
}

pub(super) fn envelope(identity: &str) -> Vec<u8> {
    format!(
        "{{\"at\":\"2020-01-02T03:04:05Z\",\"key\":\"store-identity\",\"payload\":\"{identity}\"}}"
    )
    .into_bytes()
}

pub(super) fn inject(store: &Store, records: &[(Vec<u8>, Vec<u8>)]) -> TestResult {
    let mut batch = store.db.batch();
    for (key, value) in records {
        batch.insert(&store.journal, key.clone(), value.clone());
    }
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(())
}

pub(super) fn canonical(raw: &[u8]) -> (Vec<u8>, Vec<u8>) {
    (Store::journal_key(PHASE, KEY), raw.to_vec())
}

pub(super) fn rows(
    snapshot: &Snapshot,
    keyspace: &Keyspace,
) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut records = Vec::new();
    for guard in snapshot.prefix(keyspace, b"").take(513) {
        if records.len() == 512 {
            return Err("a lineage fixture exceeded its 512-row inspection budget".into());
        }
        let (key, value) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        records.push((key.to_vec(), value.to_vec()));
    }
    Ok(records)
}

pub(super) fn lineage_rows(store: &Store) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let prefix = Store::journal_key(PHASE, "");
    Ok(rows(&store.db.snapshot(), &store.journal)?
        .into_iter()
        .filter(|(key, _)| key.starts_with(&prefix))
        .collect())
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Witness {
    pub(super) sequence: u64,
    pub(super) evidence: u64,
    pub(super) derived: u64,
    pub(super) format: StoreFormat,
    pub(super) source_digest: String,
    pub(super) all_tables_digest: String,
    pub(super) entities: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) journal: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) meta: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) receipts: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Witness {
    pub(super) fn capture(store: &Store) -> TestResult<Self> {
        let view = store.snapshot();
        let raw = store.db.snapshot();
        equal(view.sequence(), raw.seqno())?;
        Ok(Self {
            sequence: view.sequence(),
            evidence: view.evidence_generation(),
            derived: view.derived_generation(),
            format: store.store_format()?,
            source_digest: view.tables_digest(&[Table::Schools])?,
            all_tables_digest: view.tables_digest(&Table::ALL)?,
            entities: rows(&raw, &store.entities)?,
            journal: rows(&raw, &store.journal)?,
            meta: rows(&raw, &store.meta)?,
            receipts: rows(&raw, &store.receipts)?,
        })
    }

    pub(super) fn unchanged_evidence(&self, after: &Self) -> TestResult {
        equal(after.evidence, self.evidence)?;
        equal(after.derived, self.derived)?;
        equal(&after.format, &self.format)?;
        equal(&after.source_digest, &self.source_digest)?;
        equal(&after.all_tables_digest, &self.all_tables_digest)?;
        equal(&after.entities, &self.entities)?;
        equal(&after.meta, &self.meta)?;
        equal(&after.receipts, &self.receipts)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Refusal {
    Decode,
    InvalidIdentity,
    Missing,
    Multiple,
    UnexpectedKey,
    UnsupportedRepresentation,
    MalformedEnvelope,
    OversizedEnvelope { bytes: usize, max: usize },
}

pub(super) fn refused(result: StoreResult<StoreIdentity>, expected: Refusal) -> TestResult {
    match (result, expected) {
        (Err(StoreError::Decode { key, source }), Refusal::Decode) => {
            equal(key.as_str(), "export-lineage-v1/store-identity")?;
            if !source.is_syntax() && !source.is_eof() {
                return Err(format!("invalid JSON lost its syntax source: {source}").into());
            }
            Ok(())
        }
        (
            Err(StoreError::StoreIdentity(StoreIdentityError::InvalidFormat)),
            Refusal::InvalidIdentity,
        ) => Ok(()),
        (Err(StoreError::StoreLineage(StoreLineageError::Missing)), Refusal::Missing) => Ok(()),
        (Err(StoreError::StoreLineage(StoreLineageError::Multiple)), Refusal::Multiple) => Ok(()),
        (
            Err(StoreError::StoreLineage(StoreLineageError::UnexpectedKey)),
            Refusal::UnexpectedKey,
        ) => Ok(()),
        (
            Err(StoreError::StoreLineage(StoreLineageError::UnsupportedRepresentation)),
            Refusal::UnsupportedRepresentation,
        ) => Ok(()),
        (
            Err(StoreError::StoreLineage(StoreLineageError::MalformedEnvelope)),
            Refusal::MalformedEnvelope,
        ) => Ok(()),
        (
            Err(StoreError::StoreLineage(StoreLineageError::OversizedEnvelope { bytes, max })),
            Refusal::OversizedEnvelope {
                bytes: expected_bytes,
                max: expected_max,
            },
        ) => {
            equal(bytes, expected_bytes)?;
            equal(max, expected_max)
        }
        (observed, expected) => Err(format!("observed={observed:?}; expected={expected:?}").into()),
    }
}

pub(super) fn refuse_preserving(records: &[(Vec<u8>, Vec<u8>)], expected: Refusal) -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    seed(&store)?;
    inject(&store, records)?;
    let before = Witness::capture(&store)?;
    refused(store.snapshot().store_identity(), expected)?;
    refused(store.store_identity(), expected)?;
    refused(store.store_identity(), expected)?;
    equal(Witness::capture(&store)?, before)
}
