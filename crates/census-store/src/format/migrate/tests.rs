use std::path::Path;

use census_domain::model::{
    AccessBlockKind, CanonicalSchool, CoverageRow, CoverageScope, ReviewCase, ReviewState,
    SourceAccessCondition,
};
use census_domain::UsJurisdiction;
use serde::Serialize;

use super::super::{Store, StoreError};
use crate::keys::{derived_generation_prefix, derived_key, observation_key};
use crate::{generation, meta, StoreResult, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name,
        census_domain::model::normalize_name(name),
    )
    .0
}

fn erase_current_schema(store: &Store) -> TestResult<()> {
    let mut batch = store.db.batch();
    for key in [
        "schema:store_version",
        "schema:key_format",
        "schema:created_by",
        "schema:created_at",
    ] {
        batch.remove(&store.meta, key);
    }
    for table in Table::ALL {
        batch.remove(&store.meta, format!("rows:{}", table.file()));
        batch.remove(&store.meta, format!("sequence:{}", table.file()));
    }
    for key in [
        generation::ENTITIES,
        generation::DERIVED_CURRENT,
        generation::DERIVED_NEXT,
        generation::DERIVED_RECLAIM_FROM,
    ] {
        batch.remove(&store.meta, key);
    }
    batch.commit()?;
    Ok(())
}

fn raw_import<T: Serialize>(
    root: &Path,
    table: Table,
    writes: &[(&str, u64, T)],
) -> TestResult<()> {
    let (db, entities, _journal, _meta, _receipts) =
        crate::open_keyspaces(root, crate::format::DEFAULT_CACHE_BYTES)?;
    {
        let mut batch = db.batch();
        for (id, sequence, record) in writes {
            batch.insert(
                &entities,
                observation_key(table, id, *sequence),
                serde_json::to_vec(record)?,
            );
        }
        batch.commit()?;
    }
    drop(entities);
    drop(db);
    Ok(())
}

fn drop_derived_rows(root: &Path, table: Table, generation_of: u64) -> TestResult<()> {
    let (db, entities, _journal, _meta, _receipts) =
        crate::open_keyspaces(root, crate::format::DEFAULT_CACHE_BYTES)?;
    let mut batch = db.batch();
    for guard in entities.prefix(derived_generation_prefix(table, generation_of)) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        batch.remove(&entities, key.as_ref());
    }
    batch.commit()?;
    drop(entities);
    drop(db);
    Ok(())
}

fn case(id: &str, state: ReviewState) -> ReviewCase {
    ReviewCase {
        id: id.to_string(),
        family: "Athlete identity".to_string(),
        subject_id: format!("subject:{id}"),
        subject: "Candidate A vs Candidate B".to_string(),
        detail: "two profiles share a name".to_string(),
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
    let root = dir.path();
    let store = Store::open(root)?;
    store.replace_many(
        Table::Coverage,
        &[CoverageRow::new(CoverageScope::Jurisdiction, "wi")],
    )?;
    erase_current_schema(&store)?;
    drop(store);
    match Store::open(root) {
        Err(StoreError::MigrationRequired { .. }) => {}
        other => return Err(format!("expected a migration refusal: {:?}", other.err()).into()),
    }
    Ok(())
}

#[test]
fn migrating_a_legacy_store_rewrites_derived_rows_and_collapses_maps() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    let store = Store::open(root)?;
    store.replace_many(
        Table::Coverage,
        &[CoverageRow::new(CoverageScope::Jurisdiction, "wi")],
    )?;
    store.append(Table::Schools, &school("Legacy School"))?;
    erase_current_schema(&store)?;
    drop(store);

    let wi = CoverageRow::new(CoverageScope::Jurisdiction, "wi");
    let mn = CoverageRow::new(CoverageScope::Jurisdiction, "mn");
    let decided = case("case:1", ReviewState::Resolved);
    let pending = case("case:1", ReviewState::Pending);
    let early = access("2026-01-01T00:00:00Z", "first refusal");
    let late = access("2026-02-01T00:00:00Z", "second refusal");
    let access_id = late.id.clone();
    raw_import(
        root,
        Table::Coverage,
        &[(&wi.id, 0, wi.clone()), (&mn.id, 0, mn.clone())],
    )?;
    raw_import(
        root,
        Table::ReviewCases,
        &[("case:1", 3, decided.clone()), ("case:1", 7, pending)],
    )?;
    raw_import(
        root,
        Table::SourceAccess,
        &[
            (access_id.as_str(), 5, early),
            (access_id.as_str(), 9, late),
        ],
    )?;
    for table in [Table::Coverage, Table::ReviewCases, Table::SourceAccess] {
        drop_derived_rows(root, table, 0)?;
    }

    let report = Store::migrate(root)?;
    {
        let (left, right) = (&report.from_schema, &None);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&report.already_current, &false);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    if !report.integrity_ok {
        return Err("a migrated store must pass its own integrity check".into());
    }
    {
        let (left, right) = (&report.rewritten_rows, &3);
        if left != right {
            return Err(format!(
                "two coverage rows and one folded review case are rewritten — \\
                 left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&report.dropped_rows, &2);
        if left != right {
            return Err(format!(
                "the superseded copies of both a review case and an access row are folded \\
                 away — left={left:?} right={right:?}"
            )
            .into());
        }
    }

    let store = Store::open(root)?;
    {
        let (left, right) = (&store.scan::<CanonicalSchool>(Table::Schools)?.len(), &1);
        if left != right {
            return Err(format!(
                "observation rows are never rewritten — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    let mut coverage: Vec<String> = store
        .scan::<CoverageRow>(Table::Coverage)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    coverage.sort();
    {
        let (left, right) = (
            &coverage,
            &vec!["jurisdiction:mn".to_string(), "jurisdiction:wi".to_string()],
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    {
        let (left, right) = (&cases.len(), &1);
        if left != right {
            return Err(format!(
                "one row per case id after folding — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&cases[0].state, &ReviewState::Resolved);
        if left != right {
            return Err(format!(
                "a decided case survives a later pending copy — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    let access = store.scan::<SourceAccessCondition>(Table::SourceAccess)?;
    {
        let (left, right) = (&access.len(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&access[0].observed_at, &"2026-02-01T00:00:00Z".to_string());
        if left != right {
            return Err(format!(
                "a map table folds field-wise, so the later observation wins — \\
                 left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (
            &store.walk_table(Table::SourceAccess)?.foreign_sequences,
            &0,
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    check_generation_keys(
        &store,
        Table::Coverage,
        &["jurisdiction:mn", "jurisdiction:wi"],
    )?;
    {
        let (left, right) = (&store.derived_generation(), &1);
        if left != right {
            return Err(format!(
                "the rewrite publishes generation one — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    Ok(())
}

fn check_generation_keys(store: &Store, table: Table, ids: &[&str]) -> StoreResult<()> {
    for id in ids {
        let key = derived_key(table, 1, id.as_bytes());
        let present = store
            .entities
            .get(&key)
            .map_err(|source| StoreError::Read { source })?;
        if present.is_none() {
            return Err(StoreError::Invariant {
                detail: format!("{id} is not stored under generation one"),
            });
        }
    }
    Ok(())
}

#[test]
fn migrating_a_current_store_reports_it_as_current() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    {
        let store = Store::open(root)?;
        store.replace_many(
            Table::Coverage,
            &[CoverageRow::new(CoverageScope::Jurisdiction, "wi")],
        )?;
    }
    let report = Store::migrate(root)?;
    if !report.already_current || report.rewritten_rows != 0 || !report.integrity_ok {
        return Err(format!("a current store needs no rewrite: {report:?}").into());
    }
    {
        let (left, right) = (&report.from_schema, &Some(1));
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn migrating_an_empty_directory_initialises_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    let report = Store::migrate(root)?;
    if report.already_current {
        return Err("an initialised store was not current before it was written".into());
    }
    if !report.integrity_ok {
        return Err("an initialised store must pass its own integrity check".into());
    }
    let store = Store::open(root)?;
    {
        let (left, right) = (&store.derived_generation(), &0);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_newer_schema_is_refused_by_both_open_and_migrate() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    {
        let store = Store::open(root)?;
        let mut batch = store.db.batch();
        meta::put_text(&mut batch, &store.meta, "schema:store_version", "99");
        batch.commit()?;
    }
    match Store::open(root) {
        Err(StoreError::FormatNewer { .. }) => {}
        other => return Err(format!("expected a format refusal: {:?}", other.err()).into()),
    }
    let migrated = Store::migrate(root);
    if !matches!(migrated, Err(StoreError::FormatNewer { .. })) {
        return Err(format!("expected a format refusal: {migrated:?}").into());
    }
    Ok(())
}
