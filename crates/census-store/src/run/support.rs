use census_domain::model::{
    CanonicalSchool, CensusRun, CoverageRow, CoverageScope, GradYear, RunManifest, SchoolYear,
};
use census_domain::UsJurisdiction;
use fjall::{Keyspace, PersistMode, Readable, Snapshot};
use std::fmt::Debug;

use crate::{Store, StoreError, StoreFormat, StoreResult, Table};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub(super) const FIXTURE_ROOT: &str = "CENSUS_PINNED_RUN_FIXTURE_ROOT";
pub(super) const KEY: &str = "run-manifest";
pub(super) const LIMIT: usize = crate::records::codec::MAX_RECORD_BYTES;
pub(super) const HISTORICAL: &[u8] = br#" {"jurisdictions":["OH","NY","OH"],"cohort":2027,"run":{"revision":1,"season":2026},"store_identity":"ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789"} "#;

pub(super) fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

pub(super) fn fixture_dir() -> TestResult<tempfile::TempDir> {
    match std::env::var_os(FIXTURE_ROOT) {
        Some(root) => Ok(tempfile::Builder::new()
            .prefix("pinned-run-")
            .tempdir_in(root)?),
        None => Ok(tempfile::tempdir()?),
    }
}

pub(super) fn manifest(identity: &str, revision: u32) -> TestResult<RunManifest> {
    Ok(RunManifest {
        store_identity: identity.to_owned(),
        run: CensusRun::new(
            SchoolYear::new(2026).ok_or("invalid fixture season")?,
            revision,
        )
        .ok_or("invalid fixture run revision")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![
            UsJurisdiction::Ohio,
            UsJurisdiction::NewYork,
            UsJurisdiction::Ohio,
        ],
    })
}

pub(super) fn seed(store: &Store) -> TestResult {
    let school = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "Synthetic Pinned Run School",
        "synthetic pinned run school".to_owned(),
        None,
    )
    .0;
    store.append(Table::Schools, &school)?;
    store.store_identity()?;
    let mut stage = store.stage_derived()?;
    for table in Table::ALL
        .into_iter()
        .filter(|table| table.generation_partitioned())
    {
        stage.carry_forward(table)?;
    }
    stage.replace_many(
        Table::Coverage,
        &[CoverageRow::new(CoverageScope::Jurisdiction, "oh")],
    )?;
    stage.publish("pinned-run-synthetic-derived", "synthetic-derived-digest")?;
    Ok(())
}

pub(super) fn inject(store: &Store, raw: &[u8]) -> TestResult {
    let mut batch = store.db.batch();
    batch.insert(&store.meta, KEY, raw);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

pub(super) fn raw_binding(store: &Store) -> TestResult<Option<Vec<u8>>> {
    Ok(store.meta.get(KEY)?.map(|raw| raw.to_vec()))
}

pub(super) fn encoded_boundary(bytes: usize) -> TestResult<RunManifest> {
    let mut value = manifest("", 1)?;
    let overhead = serde_json::to_vec(&value)?.len();
    let count = bytes
        .checked_sub(overhead)
        .ok_or("fixture byte ceiling is below JSON overhead")?;
    value.store_identity = "x".repeat(count);
    equal(serde_json::to_vec(&value)?.len(), bytes)?;
    Ok(value)
}

pub(super) fn rows(snapshot: &Snapshot, space: &Keyspace) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut result = Vec::new();
    for guard in snapshot.prefix(space, b"").take(257) {
        if result.len() == 256 {
            return Err("pinned run fixture exceeds its 256-row inspection budget".into());
        }
        let (key, value) = guard.into_inner()?;
        result.push((key.to_vec(), value.to_vec()));
    }
    Ok(result)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Witness {
    pub(super) sequence: u64,
    pub(super) evidence: u64,
    pub(super) derived: u64,
    pub(super) format: StoreFormat,
    pub(super) digest: String,
    pub(super) entities: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) journal: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) metadata: Vec<(Vec<u8>, Vec<u8>)>,
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
            digest: view.tables_digest(&Table::ALL)?,
            entities: rows(&raw, &store.entities)?,
            journal: rows(&raw, &store.journal)?,
            metadata: rows(&raw, &store.meta)?,
            receipts: rows(&raw, &store.receipts)?,
        })
    }

    pub(super) fn binding_only(&self, after: &Self, bytes: &[u8]) -> TestResult {
        equal(
            after.sequence,
            self.sequence.checked_add(1).ok_or("sequence overflow")?,
        )?;
        equal(
            (after.evidence, after.derived),
            (self.evidence, self.derived),
        )?;
        equal(&after.format, &self.format)?;
        equal(&after.digest, &self.digest)?;
        equal(&after.entities, &self.entities)?;
        equal(&after.journal, &self.journal)?;
        equal(&after.receipts, &self.receipts)?;
        let mut metadata = self.metadata.clone();
        metadata.push((KEY.as_bytes().to_vec(), bytes.to_vec()));
        metadata.sort_by(|left, right| left.0.cmp(&right.0));
        equal(&after.metadata, &metadata)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Refusal {
    Utf8,
    Json,
    Oversized(usize),
}

pub(super) fn refused(result: StoreResult<Option<RunManifest>>, expected: Refusal) -> TestResult {
    match (result, expected) {
        (Err(StoreError::SchemaUnknown { detail }), Refusal::Utf8) if detail.contains(KEY) => {
            Ok(())
        }
        (Err(StoreError::Json { detail, .. }), Refusal::Json) if detail.contains(KEY) => Ok(()),
        (Err(StoreError::RowTooLarge { bytes, max }), Refusal::Oversized(length)) => {
            equal((bytes, max), (length, LIMIT))
        }
        (observed, expected) => Err(format!("observed={observed:?}; expected={expected:?}").into()),
    }
}

pub(super) fn oversized_write(result: StoreResult<()>) -> TestResult {
    match result {
        Err(StoreError::RowTooLarge { bytes, max }) if bytes > LIMIT => equal(max, LIMIT),
        observed => Err(format!("expected oversized first-binding refusal: {observed:?}").into()),
    }
}
