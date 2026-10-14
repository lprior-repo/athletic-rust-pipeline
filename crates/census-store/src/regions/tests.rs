use super::{Envelope, RegionRegistration, RegionRegistryError, MAX_REGION_RECORD_BYTES};
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::model::{ContractRefV1, RegionMembershipContractV1};
use fjall::{Keyspace, PersistMode, Readable, Snapshot};
use std::fmt::Debug;

mod bounds;
mod lifecycle;
mod refusals;

const CACHE_BYTES: u64 = 8_388_608;
const FIXTURE: &str =
    include_str!("../../../census-domain/tests/fixtures/region_membership/synthetic-contract.json");

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Rows = Vec<(Vec<u8>, Vec<u8>)>;

fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

fn contract(revision: u32) -> TestResult<RegionMembershipContractV1> {
    let original: RegionMembershipContractV1 = serde_json::from_str(FIXTURE)?;
    let mut input = original.input().clone();
    input.revision = revision;
    Ok(RegionMembershipContractV1::new(input)?)
}

fn key(reference: &ContractRefV1) -> String {
    format!(
        "region-contracts:v1:{}:{}",
        reference.revision(),
        reference.digest()
    )
}

fn inject(store: &Store, reference: &ContractRefV1, bytes: &[u8]) -> StoreResult<()> {
    let _appends = store.lock_appends();
    let mut batch = store.db.batch();
    batch.insert(&store.meta, key(reference).as_bytes(), bytes);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn envelope_value(contract: &RegionMembershipContractV1) -> TestResult<serde_json::Value> {
    Ok(serde_json::to_value(Envelope {
        version: 1,
        reference: &contract.reference()?,
        payload: contract,
    })?)
}

fn replace(value: &mut serde_json::Value, path: &str, new: serde_json::Value) -> TestResult {
    *value.pointer_mut(path).ok_or("missing fixture field")? = new;
    Ok(())
}

fn rows(snapshot: &Snapshot, keyspace: &Keyspace) -> TestResult<Rows> {
    let mut held = Vec::new();
    held.try_reserve_exact(512)?;
    for guard in snapshot.prefix(keyspace, b"").take(513) {
        if held.len() == 512 {
            return Err("region fixture inventory exceeds 512 rows".into());
        }
        let (key, value) = guard
            .into_inner()
            .map_err(|source| StoreError::Read { source })?;
        held.push((key.to_vec(), value.to_vec()));
    }
    Ok(held)
}

#[derive(Debug, PartialEq, Eq)]
struct Witness {
    sequence: u64,
    evidence: u64,
    derived: u64,
    tables: String,
    entities: Rows,
    journal: Rows,
    meta: Rows,
    receipts: Rows,
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
            tables: view.tables_digest(&Table::ALL)?,
            entities: rows(&raw, &store.entities)?,
            journal: rows(&raw, &store.journal)?,
            meta: rows(&raw, &store.meta)?,
            receipts: rows(&raw, &store.receipts)?,
        })
    }

    fn same_evidence(&self, other: &Self) -> TestResult {
        equal(self.evidence, other.evidence)?;
        equal(self.derived, other.derived)?;
        equal(&self.tables, &other.tables)?;
        equal(&self.entities, &other.entities)?;
        equal(&self.journal, &other.journal)?;
        equal(&self.receipts, &other.receipts)
    }
}

fn seed(store: &Store) -> TestResult {
    store.append(
        Table::Schools,
        &serde_json::json!({"id":"region-fixture-school","note":"retained"}),
    )?;
    Ok(())
}

fn failure<T: Debug>(result: Result<T, RegionRegistryError>) -> TestResult<RegionRegistryError> {
    match result {
        Err(error) => Ok(error),
        Ok(value) => Err(format!("expected refusal, admitted {value:?}").into()),
    }
}

fn unchanged_refusals(
    store: &Store,
    contract: &RegionMembershipContractV1,
) -> TestResult<[RegionRegistryError; 3]> {
    let reference = contract.reference()?;
    let before = Witness::capture(store)?;
    let errors = [
        failure(store.region_contract(&reference))?,
        failure(store.snapshot().region_contract(&reference))?,
        failure(store.register_region_contract(contract))?,
    ];
    equal(Witness::capture(store)?, before)?;
    Ok(errors)
}
