mod graph_effects;
pub(crate) use graph_effects::assert_graph_staging;

use std::path::Path;

use census_domain::model::{
    AcceptedAthleteIdentity, AthleteIdentityIndex, CanonicalAthlete, Evidence, Gender, GradYear,
    IdentityApplication, IdentityProjectionBuilder, SchoolId, SourceIdentity, SourceNamespace,
    SourceRef,
};
use fjall::{Database, Keyspace, KeyspaceCreateOptions};

use super::*;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type PhysicalRows = Vec<(Vec<u8>, Vec<u8>)>;
pub(crate) type PhysicalState = Vec<(String, PhysicalRows)>;

pub(crate) fn fixture() -> TestResult<(tempfile::TempDir, Store, crate::PublicIdentityWriterV1)> {
    let (directory, store, writer) = unenrolled_fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

pub(crate) fn unenrolled_fixture(
) -> TestResult<(tempfile::TempDir, Store, crate::PublicIdentityWriterV1)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("identity");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    Ok((directory, store, writer))
}

fn rows(keyspace: &Keyspace) -> TestResult<PhysicalRows> {
    let rows = keyspace
        .iter()
        .take(2_049)
        .map(|guard| {
            let (key, value) = guard.into_inner()?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect::<TestResult<PhysicalRows>>()?;
    check!(
        rows.len() <= 2_048,
        "fixture exceeded physical-row verification bound"
    );
    Ok(rows)
}

pub(crate) fn retained_state(store: &Store) -> TestResult<PhysicalState> {
    [
        (crate::ENTITIES, &store.entities),
        (crate::JOURNAL, &store.journal),
        (crate::META, &store.meta),
        (crate::RECEIPTS, &store.receipts),
    ]
    .into_iter()
    .map(|(name, keyspace)| Ok((name.to_owned(), rows(keyspace)?)))
    .collect()
}

pub(crate) fn physical_state(root: &Path) -> TestResult<PhysicalState> {
    check!(root.join(crate::DB_DIR).join("version").is_file());
    let db = Database::builder(root.join(crate::DB_DIR))
        .cache_size(8_388_608)
        .open()?;
    [
        crate::ENTITIES,
        crate::JOURNAL,
        crate::META,
        crate::RECEIPTS,
    ]
    .into_iter()
    .map(|name| {
        check!(db.keyspace_exists(name), "missing fixture keyspace {name}");
        let keyspace = db.keyspace(name, KeyspaceCreateOptions::default)?;
        Ok((name.to_owned(), rows(&keyspace)?))
    })
    .collect()
}

pub(crate) fn subject(label: &str, source_id: &str) -> TestResult<CanonicalAthlete> {
    let school = SchoolId::mint("sch", &["public-identity-fixture", label]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        format!("Synthetic Runner {label}"),
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, source_id),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "public-identity-fixture",
            Some(format!("https://mi.milesplit.com/athletes/{source_id}")),
        ),
        "2026-10-14",
    ));
    Ok(athlete)
}

pub(super) fn accepted(athlete: &CanonicalAthlete) -> TestResult<AcceptedAthleteIdentity> {
    let mut index = AthleteIdentityIndex::default();
    index.observe(athlete)?;
    let builder = IdentityProjectionBuilder::new(index, &[], &[])?;
    let application = builder
        .source_applications("2026-10-14")
        .next()
        .ok_or("fixture lacks source application")??;
    match application {
        IdentityApplication::Accepted(decision) => Ok(decision),
        IdentityApplication::Retained(issue) => Err(format!("fixture retained: {issue:?}").into()),
    }
}

pub(super) fn refused<T>(result: StoreResult<T>) -> TestResult<StoreError> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("expected public-identity qualification refusal".into()),
    }
}
