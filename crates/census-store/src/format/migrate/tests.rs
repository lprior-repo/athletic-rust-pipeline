use std::path::Path;

use census_domain::model::{
    AccessBlockKind, CanonicalSchool, CoverageRow, CoverageScope, ReviewCase, ReviewState,
    SourceAccessCondition,
};
use census_domain::UsJurisdiction;
use serde::Serialize;

use crate::keys::observation_key;
use crate::{generation, meta, Store, StoreError, Table};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod legacy;

pub(super) fn erase_current_schema(store: &Store) -> TestResult {
    let mut batch = store.db.batch();
    [
        "schema:store_version",
        "schema:key_format",
        "schema:created_by",
        "schema:created_at",
        generation::ENTITIES,
        generation::DERIVED_CURRENT,
        generation::DERIVED_NEXT,
        generation::DERIVED_RECLAIM_FROM,
    ]
    .into_iter()
    .for_each(|key| batch.remove(&store.meta, key));
    Table::ALL.into_iter().for_each(|table| {
        batch.remove(&store.meta, format!("rows:{}", table.file()));
        batch.remove(&store.meta, format!("sequence:{}", table.file()));
    });
    batch.commit()?;
    Ok(())
}

pub(super) fn raw_import<T: Serialize>(
    root: &Path,
    table: Table,
    writes: &[(&str, u64, T)],
) -> TestResult {
    let (db, entities, _, _, _) = crate::open_keyspaces(root, crate::format::DEFAULT_CACHE_BYTES)?;
    let mut batch = db.batch();
    writes
        .iter()
        .try_for_each(|(id, sequence, record)| -> TestResult {
            batch.insert(
                &entities,
                observation_key(table, id, *sequence),
                serde_json::to_vec(record)?,
            );
            Ok(())
        })?;
    batch.commit()?;
    Ok(())
}

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name,
        census_domain::model::normalize_name(name),
        None,
    )
    .0
}

fn case(id: &str, state: ReviewState) -> ReviewCase {
    ReviewCase {
        id: id.into(),
        family: "Athlete identity".into(),
        subject_id: format!("subject:{id}"),
        subject: "Candidate A vs Candidate B".into(),
        detail: "two profiles share a name".into(),
        state,
        member_ids: Vec::new(),
    }
}

fn access(observed_at: &str, detail: &str) -> SourceAccessCondition {
    SourceAccessCondition::new(
        "athleticnet",
        "www.athletic.net",
        AccessBlockKind::RateLimited,
        429,
        observed_at,
        detail,
    )
}

#[test]
fn opening_a_legacy_store_refuses_and_names_the_migration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.replace_many(
        Table::Coverage,
        &[CoverageRow::new(CoverageScope::Jurisdiction, "wi")],
    )?;
    erase_current_schema(&store)?;
    drop(store);
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::MigrationRequired { .. })
    ));
    Ok(())
}

#[test]
fn migrating_a_current_store_reports_it_as_current() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.replace_many(
        Table::Coverage,
        &[CoverageRow::new(CoverageScope::Jurisdiction, "wi")],
    )?;
    drop(store);
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.already_current, report.rewritten_rows, report.dropped_rows, report.from_schema), (true, 0, 0, Some(2)));
    check!(report.integrity_ok);
    Ok(())
}

#[test]
fn migrating_an_empty_directory_initialises_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let report = Store::migrate(dir.path())?;
    check!(!report.already_current);
    check!(report.integrity_ok);
    check!(eq; Store::open(dir.path())?.derived_generation(), 0);
    check!(eq; Store::format(dir.path())?.schema_version, Some(2));
    Ok(())
}

#[test]
fn a_newer_schema_is_refused_by_both_open_and_migrate() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut batch = store.db.batch();
    meta::put_text(&mut batch, &store.meta, "schema:store_version", "99");
    batch.commit()?;
    drop(store);
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::FormatNewer { found: 99, .. })
    ));
    check!(matches!(
        Store::migrate(dir.path()),
        Err(StoreError::FormatNewer { found: 99, .. })
    ));
    Ok(())
}
