use std::sync::Arc;

use super::sequences::mark_key;
use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None).0
}

fn observed(name: &str, source: &str, at: &str) -> CanonicalSchool {
    let mut row = school(name);
    row.evidence
        .push(Evidence::parsed(SourceRef::id(source), at));
    row
}

fn mark(store: &Store, table: Table) -> TestResult<Option<u64>> {
    let Some(value) = store.meta.get(mark_key(table))? else {
        return Ok(None);
    };
    Ok(Some(std::str::from_utf8(&value)?.trim().parse()?))
}

fn counter(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store
        .stats()?
        .appended
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, next)| next)
        .ok_or("table append counter")?)
}

#[test]
fn a_batch_stores_the_mark_its_sequences_land_in() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    check!(eq; mark(&store, Table::Schools)?, Some(0), "an open leaves every table with a mark");
    store.append_many(
        Table::Schools,
        &[
            observed("Abbotsford", "wiaa_schools", "2026-09-19"),
            observed("Colby", "wiaa_schools", "2026-09-19"),
        ],
    )?;
    check!(eq; mark(&store, Table::Schools)?, Some(2));
    check!(eq; counter(&store, Table::Schools)?, 2);
    let oversized = serde_json::json!({ "id": "s".repeat(MAX_ID_BYTES + 1) });
    check!(store.append(Table::Schools, &oversized).is_err());
    check!(eq; mark(&store, Table::Schools)?, Some(2), "a refused batch may not move the mark");
    check!(eq; counter(&store, Table::Schools)?, 2);
    Ok(())
}

#[test]
fn a_reopened_store_resumes_at_the_mark_its_last_batch_committed() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        store.append(
            Table::Schools,
            &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
        )?;
        check!(eq; mark(&store, Table::Schools)?, Some(1));
    }
    let store = Store::open(dir.path())?;
    check!(eq; mark(&store, Table::Schools)?, Some(1), "the mark survives the reopen");
    check!(eq; counter(&store, Table::Schools)?, 1, "the same next sequence the last batch left");
    let mut again = observed("Abbotsford", "mshsl_schools", "2026-09-20");
    again.city = Some("Abbotsford".into());
    store.append(Table::Schools, &again)?;
    check!(eq; mark(&store, Table::Schools)?, Some(2));
    let rows = store.scan::<CanonicalSchool>(Table::Schools)?;
    let resumed = rows
        .iter()
        .find(|row| row.id == again.id)
        .ok_or("resumed school")?;
    check!(eq; resumed.evidence.len(), 2, "the resumed append must not overwrite the first observation");
    check!(eq; resumed.city.as_deref(), Some("Abbotsford"));
    Ok(())
}

#[test]
fn a_store_written_without_marks_learns_them_from_one_scan() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        store.append(
            Table::Schools,
            &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
        )?;
        store.meta.remove(mark_key(Table::Schools))?;
        store.meta.remove(mark_key(Table::Athletes))?;
        store.flush()?;
    }
    {
        let store = Store::open(dir.path())?;
        check!(eq; mark(&store, Table::Schools)?, Some(1), "the open derived the mark it was missing");
        check!(eq; mark(&store, Table::Athletes)?, Some(0), "an empty table is marked zero");
        check!(eq; counter(&store, Table::Schools)?, 1);
        let mut again = observed("Abbotsford", "mshsl_schools", "2026-09-20");
        again.city = Some("Abbotsford".into());
        store.append(Table::Schools, &again)?;
        check!(eq; mark(&store, Table::Schools)?, Some(2));
        let rows = store.scan::<CanonicalSchool>(Table::Schools)?;
        let resumed = rows
            .iter()
            .find(|row| row.id == again.id)
            .ok_or("resumed school")?;
        check!(eq; resumed.evidence.len(), 2, "the migration resumes at the sequence the scan found");
    }
    let store = Store::open(dir.path())?;
    check!(eq; mark(&store, Table::Schools)?, Some(2));
    check!(eq; counter(&store, Table::Schools)?, 2);
    Ok(())
}

#[test]
fn an_open_with_a_mark_never_walks_the_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        store.append(
            Table::Schools,
            &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
        )?;
        store
            .entities
            .insert(b"schools\0broken".as_slice(), b"{}".as_slice())?;
    }
    let store = Store::open(dir.path())?;
    check!(eq; counter(&store, Table::Schools)?, 1);
    store.meta.remove(mark_key(Table::Schools))?;
    store.flush()?;
    drop(store);
    match Store::open(dir.path()) {
        Err(StoreError::Invariant { detail }) => check!(
            detail.contains("malformed observation key"),
            "unexpected detail: {detail}"
        ),
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("expected the walk to refuse the unparseable key".into()),
    }
    Ok(())
}

#[test]
fn concurrent_appends_leave_a_mark_a_reopen_resumes_from() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    let writers = 4_usize;
    let per_writer = 25_usize;
    let mut threads = Vec::with_capacity(writers);
    for writer in 0..writers {
        let store = Arc::clone(&store);
        threads.push(std::thread::spawn(move || -> TestResult {
            for index in 0..per_writer {
                store.append(Table::Schools, &school(&format!("School {writer} {index}")))?;
            }
            Ok(())
        }));
    }
    let mut joined: TestResult = Ok(());
    for thread in threads {
        let outcome = match thread.join() {
            Ok(outcome) => outcome,
            Err(_) => Err("school writer panicked".into()),
        };
        joined = joined.and(outcome);
    }
    joined?;
    let appended = u64::try_from(writers * per_writer)?;
    check!(eq; mark(&store, Table::Schools)?, Some(appended));
    drop(store);
    let store = Store::open(dir.path())?;
    check!(eq; mark(&store, Table::Schools)?, Some(appended), "the last commit left the table's high-water mark");
    check!(eq; counter(&store, Table::Schools)?, appended);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), usize::try_from(appended)?, "no append may overwrite another writer's observation");
    Ok(())
}
