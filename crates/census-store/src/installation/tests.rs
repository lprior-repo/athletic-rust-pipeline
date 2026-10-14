use super::*;
use crate::{HistoricalStateKind, Store, StoreError, Table};
use fjall::{Keyspace, Readable, Snapshot};

#[path = "tests/artifact_eligibility.rs"]
mod artifact_eligibility;
#[path = "tests/control_ownership.rs"]
mod control_ownership;
#[path = "tests/fixture.rs"]
mod fixture;
#[path = "tests/format_corruption.rs"]
mod format_corruption;
#[path = "tests/lifecycle.rs"]
mod lifecycle;
#[path = "tests/preservation.rs"]
mod preservation;

use fixture::{configuration, equal, profile, TestResult};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Witness {
    sequence: u64,
    evidence: u64,
    derived: u64,
    rows: Vec<Vec<(Vec<u8>, Vec<u8>)>>,
}

fn rows(snapshot: &Snapshot, keyspace: &Keyspace) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(512)?;
    for row in snapshot.prefix(keyspace, b"").take(513) {
        if rows.len() == 512 {
            return Err("fixture row budget exceeded".into());
        }
        let (key, value) = row.into_inner()?;
        rows.push((key.to_vec(), value.to_vec()));
    }
    Ok(rows)
}

fn witness(store: &Store) -> TestResult<Witness> {
    let snapshot = store.db.snapshot();
    Ok(Witness {
        sequence: snapshot.seqno(),
        evidence: store.evidence_generation(),
        derived: store.derived_generation(),
        rows: vec![
            rows(&snapshot, &store.entities)?,
            rows(&snapshot, &store.journal)?,
            rows(&snapshot, &store.meta)?,
            rows(&snapshot, &store.receipts)?,
        ],
    })
}

fn prepare(root: &std::path::Path) -> TestResult<census_domain::model::InstallationConfigV1> {
    let store = Store::open_with(root, 8_388_608)?;
    let profile = profile()?;
    store.register_profile(&profile)?;
    store.flush()?;
    configuration(&profile)
}

#[test]
fn public_enrollment_path_completes_without_recursive_append_lock_and_is_immutable() -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    let enrolled = match control.enroll(&config)? {
        InstallationEnrollmentOutcome::Enrolled(enrollment) => enrollment,
        other => return Err(format!("first enrollment={other:?}").into()),
    };
    let after = witness(&control.store)?;
    equal(
        after.sequence,
        before.sequence.checked_add(2).ok_or("sequence")?,
    )?;
    equal(control.store.evidence_generation(), 0)?;
    equal(control.store.derived_generation(), 0)?;
    equal(control.store.store_format()?.lifecycle_revision, Some(3))?;
    equal(
        control.enroll(&config)?,
        InstallationEnrollmentOutcome::Repeated(enrolled.clone()),
    )?;
    equal(witness(&control.store)?, after.clone())?;
    let changed = census_domain::model::InstallationConfigV1::new(
        config.initial_profile_id().clone(),
        census_domain::model::ServiceResourceProfileV1::new(
            9,
            config.service_resources().memory_budget_bytes(),
            30,
        )?,
        config.source_registry().clone(),
        config.operating_defaults().clone(),
        config.capability(),
    );
    if !matches!(
        control.enroll(&changed),
        Err(StoreError::Installation(
            InstallationError::ImmutableConflict
        ))
    ) {
        return Err("changed configuration did not conflict".into());
    }
    equal(witness(&control.store)?, after.clone())?;
    let expected = InstallationState::Enrolled {
        enrollment: enrolled,
        config,
    };
    equal(control.show()?, expected.clone())?;
    drop(control);
    let control = Store::installation_control_existing(root.path())?;
    equal(control.show()?, expected)
}

#[test]
fn enrollment_with_preexisting_uppercase_identity_has_exactly_one_commit() -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let store = Store::open_existing(root.path())?;
    let identity = census_domain::model::StoreIdentity::parse(&"A".repeat(64))?;
    store.journal_done("export-lineage-v1", "store-identity", &identity)?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    let enrollment = match control.enroll(&config)? {
        InstallationEnrollmentOutcome::Enrolled(enrollment) => enrollment,
        other => return Err(format!("first enrollment={other:?}").into()),
    };
    equal(enrollment.store_identity(), &identity)?;
    let after = witness(&control.store)?;
    equal(
        after.sequence,
        before.sequence.checked_add(1).ok_or("sequence")?,
    )?;
    equal(before.rows.get(1), after.rows.get(1))?;
    equal(before.rows.get(0), after.rows.get(0))?;
    equal(before.rows.get(3), after.rows.get(3))
}

#[test]
fn historical_state_families_refuse_without_identity_or_other_logical_writes() -> TestResult {
    for (family, expected) in [
        ("entities", HistoricalStateKind::Entities),
        ("derived", HistoricalStateKind::Entities),
        ("receipts", HistoricalStateKind::Receipts),
        ("journal", HistoricalStateKind::Journal),
        ("run", HistoricalStateKind::LegacyRun),
        ("workflow", HistoricalStateKind::Metadata),
        ("publication", HistoricalStateKind::Metadata),
        ("seal", HistoricalStateKind::Metadata),
        ("unknown", HistoricalStateKind::Metadata),
        ("rows", HistoricalStateKind::Generations),
        ("sequence", HistoricalStateKind::Generations),
        ("generation", HistoricalStateKind::Generations),
    ] {
        let root = tempfile::tempdir()?;
        let config = prepare(root.path())?;
        let store = Store::open_existing(root.path())?;
        match family {
            "entities" => {
                store.entities.insert(b"observation", b"retained")?;
            }
            "derived" => {
                store.entities.insert(b"derived:1:row", b"retained")?;
            }
            "receipts" => {
                store.receipts.insert(b"effect", b"retained")?;
            }
            "journal" => {
                store.journal_done("work", "partial", &"retained")?;
            }
            "run" => {
                store.meta.insert(b"run-manifest", b"retained")?;
            }
            "workflow" => {
                store.meta.insert(b"record:unit:partial", b"retained")?;
            }
            "publication" => {
                store.meta.insert(b"publication:current", b"retained")?;
            }
            "seal" => {
                store.meta.insert(b"seal:current", b"retained")?;
            }
            "unknown" => {
                store.meta.insert(b"unknown", b"retained")?;
            }
            "rows" => {
                store
                    .meta
                    .insert(format!("rows:{}", Table::Schools.file()), b"1")?;
            }
            "sequence" => {
                store
                    .meta
                    .insert(format!("sequence:{}", Table::Schools.file()), b"1")?;
            }
            "generation" => {
                store.meta.insert(b"generation:derived:next", b"2")?;
            }
            _ => return Err("unknown fixture family".into()),
        }
        store.flush()?;
        drop(store);
        let mut control = Store::installation_control_existing(root.path())?;
        let before = witness(&control.store)?;
        match control.enroll(&config) {
            Err(StoreError::Installation(InstallationError::HistoricalStatePresent { kind })) => {
                equal(kind, expected)?
            }
            other => return Err(format!("{family} enrollment={other:?}").into()),
        }
        equal(witness(&control.store)?, before.clone())?;
        equal(
            crate::lineage::read(&control.store.db.snapshot(), &control.store.journal)
                .map(|identity| matches!(identity, crate::lineage::RetainedIdentity::Absent))?,
            true,
        )?;
    }
    Ok(())
}

#[test]
fn every_preexisting_profile_registry_record_is_validated_before_identity_creation() -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let store = Store::open_existing(root.path())?;
    store
        .meta
        .insert(format!("profiles:v1:{}", "c".repeat(64)), b"{")?;
    store.flush()?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    if !matches!(
        control.enroll(&config),
        Err(StoreError::ProfileRegistry(
            crate::ProfileRegistryError::Malformed { .. }
        ))
    ) {
        return Err("malformed unselected registry record accepted".into());
    }
    equal(witness(&control.store)?, before.clone())
}

#[test]
fn missing_profile_and_unsupported_profile_contract_refuse_without_lineage_creation() -> TestResult
{
    let root = tempfile::tempdir()?;
    let mut raw = profile()?.input().clone();
    raw.source_plan = census_domain::model::ContractRefV1::new(1, 2, &"a".repeat(64))?;
    let bad_profile = census_domain::model::QueryProfileV1::new(raw)?;
    let store = Store::open(root.path())?;
    let config = configuration(&bad_profile)?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    if !matches!(
        control.enroll(&config),
        Err(StoreError::ProfileRegistry(
            crate::ProfileRegistryError::MissingProfile { .. }
        ))
    ) {
        return Err("missing profile accepted".into());
    }
    equal(witness(&control.store)?, before.clone())?;
    drop(control);
    let store = Store::open_existing(root.path())?;
    store.register_profile(&bad_profile)?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    if !matches!(
        control.enroll(&config),
        Err(StoreError::Installation(
            InstallationError::ProfileExecution(_)
        ))
    ) {
        return Err("unsupported profile compiler reference accepted".into());
    }
    equal(witness(&control.store)?, before.clone())
}
