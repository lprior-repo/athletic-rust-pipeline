use super::*;
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::model::{
    ContractRefV1, Evidence, Gender, Id, ProfileDate, ProfileDateRangeV1, RegionAssertionInputV1,
    RegionAssertionV1, RegionContextV1, RegionEffectiveV1, RegionEvidenceInputV1, RegionEvidenceV1,
    RegionKeyV1, RegionMembershipContractInputV1, RegionMembershipContractV1, RegionSchoolV1,
    RegionStatementV1, RegionSubjectV1, SchoolYear, SourceRef, Sport, StoreIdentity,
    MAX_REGION_PAYLOAD_BYTES,
};
use census_domain::UsJurisdiction;
use fjall::{Keyspace, PersistMode, Readable, Snapshot};
use std::fmt::Debug;

mod bounds;
mod corruption;
mod lineage_preflight;
mod persistence;
mod pinned;
mod review_regressions;
mod versions;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

fn range() -> TestResult<ProfileDateRangeV1> {
    Ok(ProfileDateRangeV1::new(
        ProfileDate::parse("2026-01-01")?,
        ProfileDate::parse("2027-12-31")?,
    )?)
}

fn assertion(index: usize, note: &str) -> TestResult<RegionAssertionV1> {
    let school = RegionSchoolV1::new(
        Id::mint("sch", &["synthetic-registry", &index.to_string()]),
        UsJurisdiction::Michigan,
    )?;
    let mut evidence = Evidence::parsed(
        SourceRef::new(
            "synthetic-registry",
            Some("https://example.invalid/region".to_string()),
        ),
        "2026-10-01T12:00:00Z",
    );
    evidence.note = Some(note.to_string());
    Ok(RegionAssertionV1::new(RegionAssertionInputV1 {
        subject: RegionSubjectV1::School(school),
        context: RegionContextV1 {
            sport: Sport::CrossCountry,
            category: Gender::Boys,
            school_year: SchoolYear::new(2026).ok_or("school year")?,
        },
        effective: RegionEffectiveV1::Inclusive(range()?),
        statement: RegionStatementV1::Member,
        provenance: RegionEvidenceV1::new(RegionEvidenceInputV1 {
            evidence,
            capture_sha256: "a".repeat(64),
            locator: format!("synthetic-row:{index}"),
            statement: format!("Synthetic member assertion {index}"),
        })?,
    }))
}

fn input(
    revision: u32,
    assertions: Vec<RegionAssertionV1>,
) -> TestResult<RegionMembershipContractInputV1> {
    Ok(RegionMembershipContractInputV1 {
        version: 1,
        revision,
        region: RegionKeyV1::parse("synthetic:registry")?,
        jurisdictions: vec![UsJurisdiction::Michigan],
        effective: range()?,
        assertions,
    })
}

fn contract(revision: u32) -> TestResult<RegionMembershipContractV1> {
    Ok(RegionMembershipContractV1::new(input(
        revision,
        vec![assertion(1, "retained freshness")?],
    )?)?)
}

fn sized_input(target: usize) -> TestResult<RegionMembershipContractInputV1> {
    let count = target.div_ceil(3000).min(512);
    let assertions = (0..count)
        .map(|index| assertion(index, "x"))
        .collect::<TestResult<Vec<_>>>()?;
    let mut input = input(7, assertions)?;
    let initial = serde_json::to_vec(&input)?.len();
    let mut remaining = target
        .checked_sub(initial)
        .ok_or("target below synthetic header")?;
    for held in &mut input.assertions {
        let extra = remaining.min(4095);
        remaining = remaining.checked_sub(extra).ok_or("padding accounting")?;
        let mut assertion = held.input().clone();
        let mut provenance = assertion.provenance.input().clone();
        provenance.evidence.note =
            Some("x".repeat(extra.checked_add(1).ok_or("padding overflow")?));
        assertion.provenance = RegionEvidenceV1::new(provenance)?;
        *held = RegionAssertionV1::new(assertion);
    }
    equal(remaining, 0)?;
    equal(serde_json::to_vec(&input)?.len(), target)?;
    Ok(input)
}

fn rows(snapshot: &Snapshot, keyspace: &Keyspace) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(256)?;
    for guard in snapshot.prefix(keyspace, b"").take(257) {
        if rows.len() == 256 {
            return Err("fixture exceeds 256 metadata rows".into());
        }
        let (key, value) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        rows.push((key.to_vec(), value.to_vec()));
    }
    Ok(rows)
}

#[derive(Debug, PartialEq, Eq)]
struct Witness {
    sequence: u64,
    evidence: u64,
    derived: u64,
    table_digest: String,
    entities: Vec<(Vec<u8>, Vec<u8>)>,
    journal: Vec<(Vec<u8>, Vec<u8>)>,
    meta: Vec<(Vec<u8>, Vec<u8>)>,
    receipts: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Witness {
    fn capture(store: &Store) -> TestResult<Self> {
        let view = store.snapshot();
        let raw = store.db.snapshot();
        equal(view.sequence(), raw.seqno())?;
        Ok(Self {
            sequence: view.sequence(),
            evidence: view.evidence_generation(),
            derived: view.derived_generation(),
            table_digest: view.tables_digest(&Table::ALL)?,
            entities: rows(&raw, &store.entities)?,
            journal: rows(&raw, &store.journal)?,
            meta: rows(&raw, &store.meta)?,
            receipts: rows(&raw, &store.receipts)?,
        })
    }

    fn unchanged_evidence(&self, other: &Self) -> TestResult {
        equal(self.evidence, other.evidence)?;
        equal(self.derived, other.derived)?;
        equal(&self.table_digest, &other.table_digest)?;
        equal(&self.entities, &other.entities)?;
        equal(&self.journal, &other.journal)?;
        equal(&self.receipts, &other.receipts)
    }
}

fn inject(store: &Store, key: &[u8], bytes: &[u8]) -> StoreResult<()> {
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.insert(&store.meta, key, bytes);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn remove(store: &Store, key: &[u8]) -> StoreResult<()> {
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.remove(&store.meta, key);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn descriptor_value(store: &Store, reference: &ContractRefV1) -> TestResult<serde_json::Value> {
    let key = keys::descriptor(reference)?;
    let raw = store
        .meta
        .get(key.as_bytes())?
        .ok_or("missing test descriptor")?;
    Ok(serde_json::from_slice(raw.as_ref())?)
}

fn inject_payload(store: &Store, reference: &ContractRefV1, bytes: &[u8]) -> TestResult {
    let descriptor = wire::Descriptor {
        version: 1,
        reference: reference.clone(),
        encoding: "canonical_json".to_string(),
        payload_bytes: bytes.len(),
        chunk_bytes: MAX_REGION_CONTRACT_RECORD_BYTES,
        chunks: bytes.len().div_ceil(MAX_REGION_CONTRACT_RECORD_BYTES),
    };
    let key = keys::descriptor(reference)?;
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.insert(
        &store.meta,
        key.as_bytes(),
        serde_json::to_vec(&descriptor)?,
    );
    for (index, bytes) in bytes.chunks(MAX_REGION_CONTRACT_RECORD_BYTES).enumerate() {
        batch.insert(&store.meta, keys::chunk(&key, index)?, bytes);
    }
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}
