//! The measured pipeline phases: one append pass over the corpus, the merged snapshot it is read
//! back through, and one timed pass per reader — both census scopes, the best-mark reduction and
//! the workbook.
//!
//! Every phase times its own work, reports the measurement through `measure`, and asserts the
//! counts it produced against the corpus before its rate is trusted.

use anyhow::{Context, Result};
use census_store::{Store, Table};
use midwest_census::report::{self, Scope};
use midwest_census::{bests, census, workbook};
use serde::Serialize;
use std::time::Instant;
use tempfile::TempDir;

use super::corpus::{Corpus, PERFORMANCES_PER_ATHLETE};
use super::{measure, Phase};

/// Rows per `append_many` call; keeps one batch bounded regardless of corpus size.
const APPEND_BATCH: usize = 1_000;

/// Append every table, then require the store's own observation counts to match the corpus.
pub(super) fn append_corpus(store: &Store, corpus: &Corpus) -> Result<Phase> {
    let started = Instant::now();
    append_all(store, Table::Schools, &corpus.schools)?;
    append_all(store, Table::Teams, &corpus.teams)?;
    append_all(store, Table::Athletes, &corpus.athletes)?;
    append_all(store, Table::Meets, &corpus.meets)?;
    append_all(store, Table::Events, &corpus.events)?;
    append_all(store, Table::Performances, &corpus.performances)?;
    let phase = measure("append", corpus.appended_rows(), "rows", started.elapsed())?;
    expect_observations(store, corpus)?;
    Ok(phase)
}

/// One `append_many` per `APPEND_BATCH` rows.
fn append_all<T: Serialize>(store: &Store, table: Table, rows: &[T]) -> Result<()> {
    for chunk in rows.chunks(APPEND_BATCH) {
        store
            .append_many(table, chunk)
            .with_context(|| format!("appending to {}", table.file()))?;
    }
    Ok(())
}

fn expect_observations(store: &Store, corpus: &Corpus) -> Result<()> {
    let expected = [
        (Table::Schools, corpus.schools.len()),
        (Table::Teams, corpus.teams.len()),
        (Table::Athletes, corpus.athletes.len()),
        (Table::Meets, corpus.meets.len()),
        (Table::Events, corpus.events.len()),
        (Table::Performances, corpus.performances.len()),
    ];
    let stats = store.stats().context("reading store stats")?;
    for (table, rows) in expected {
        let found = stats
            .tables
            .iter()
            .find(|(name, _)| name == table.file())
            .map(|(_, count)| *count)
            .unwrap_or(0);
        let rows = u64::try_from(rows).context("row count does not fit u64")?;
        anyhow::ensure!(
            found == rows,
            "{} holds {found} observations, expected {rows}",
            table.file()
        );
    }
    Ok(())
}

/// Merge every append log into `out/`, then require the merged counts to match the corpus.
pub(super) fn consolidate_phase(store: &Store, corpus: &Corpus) -> Result<Phase> {
    let started = Instant::now();
    let counts = census::consolidate(store).context("consolidating the append logs")?;
    let written = merged_rows(&counts);
    let phase = measure("consolidate", written, "entities", started.elapsed())?;
    ensure_count(&counts, "schools", corpus.schools.len())?;
    ensure_count(&counts, "teams", corpus.teams.len())?;
    ensure_count(&counts, "athletes", corpus.athletes.len())?;
    ensure_count(&counts, "meets", corpus.meets.len())?;
    ensure_count(&counts, "events", corpus.distinct_events.len())?;
    ensure_count(&counts, "performances", corpus.performances.len())?;
    Ok(phase)
}

/// Rows over the seven real tables (the consolidated report also carries a `coaches_email_withheld`
/// counter, which is not a table).
fn merged_rows(counts: &[(String, usize)]) -> usize {
    counts
        .iter()
        .filter(|(table, _)| Table::ALL.iter().any(|known| known.file() == table))
        .map(|(_, count)| *count)
        .sum()
}

fn ensure_count(counts: &[(String, usize)], table: &str, expected: usize) -> Result<()> {
    let found = counts
        .iter()
        .find(|(name, _)| name == table)
        .map(|(_, count)| *count)
        .with_context(|| format!("consolidate did not report a count for {table}"))?;
    anyhow::ensure!(
        found == expected,
        "{table}: consolidated {found} rows, expected {expected}"
    );
    Ok(())
}

/// Both report scopes over the same consolidated snapshot.
pub(super) fn census_phases(store: &Store, cohort: usize) -> Result<(Phase, Phase)> {
    let started = Instant::now();
    let core = report::build_census(store, Scope::Core).context("building the core census")?;
    let core_phase = measure("census_core", cohort, "athletes", started.elapsed())?;
    anyhow::ensure!(
        core.totals.athletes == cohort,
        "core census counted {} athletes, expected {cohort}",
        core.totals.athletes
    );
    anyhow::ensure!(
        core.totals.class_of_2027 == cohort,
        "core census counted {} class-of-2027 athletes, expected {cohort}",
        core.totals.class_of_2027
    );

    let started = Instant::now();
    let all_sources = report::build_census(store, Scope::AllSources)
        .context("building the all-sources census")?;
    let all_sources_phase = measure("census_all_sources", cohort, "athletes", started.elapsed())?;
    anyhow::ensure!(
        all_sources.totals.athletes == cohort,
        "all-sources census counted {} athletes, expected {cohort}",
        all_sources.totals.athletes
    );
    Ok((core_phase, all_sources_phase))
}

/// Reduce best marks and require exactly one row per athlete, each resting on both marks.
pub(super) fn bests_phase(
    store: &Store,
    athletes: usize,
    performances: usize,
) -> Result<(Phase, usize)> {
    let started = Instant::now();
    let rows = bests::build(
        store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .context("reducing best marks")?;
    let phase = measure("bests", performances, "performances", started.elapsed())?;
    anyhow::ensure!(
        rows.len() == athletes,
        "bests produced {} rows, expected {athletes}",
        rows.len()
    );
    for row in &rows {
        anyhow::ensure!(
            row.marks_in_event == PERFORMANCES_PER_ATHLETE,
            "athlete {} reduced {} marks, expected {}",
            row.athlete_id,
            row.marks_in_event,
            PERFORMANCES_PER_ATHLETE
        );
    }
    Ok((phase, rows.len()))
}

/// Build the workbook into the temporary directory and check the artifact is a real xlsx. The item
/// count is the number of merged rows the workbook reads, not the append count.
pub(super) fn workbook_phase(store: &Store, dir: &TempDir, entities: usize) -> Result<Phase> {
    let out = dir.path().join("synthetic-census.xlsx");
    let started = Instant::now();
    let path = workbook::build(
        store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(out.clone()),
            limit: None,
            scope: Scope::Core,
        },
    )
    .context("building the workbook")?;
    let phase = measure("workbook", entities, "entities", started.elapsed())?;
    anyhow::ensure!(
        path == out,
        "workbook was written to {} instead of {}",
        path.display(),
        out.display()
    );
    let bytes = std::fs::read(&path).context("reading the workbook back")?;
    anyhow::ensure!(
        bytes.starts_with(b"PK\x03\x04"),
        "workbook {} is not an xlsx (zip) container",
        path.display()
    );
    let eocd = bytes
        .len()
        .checked_sub(22)
        .context("workbook is too short to carry a zip end-of-central-directory record")?;
    let signature = bytes
        .get(eocd..eocd.saturating_add(4))
        .context("reading the zip end-of-central-directory signature")?;
    anyhow::ensure!(
        signature == b"PK\x05\x06".as_slice(),
        "workbook {} is a truncated zip container",
        path.display()
    );
    println!("metric=workbook_bytes value={} unit=bytes", bytes.len());
    println!("note=workbook path {}", path.display());
    Ok(phase)
}
