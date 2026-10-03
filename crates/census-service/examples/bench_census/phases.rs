use anyhow::{Context, Result};
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation, Scope};
use census_report::{bests, workbook};
use census_service::census;
use census_store::{Store, Table};
use serde::Serialize;
use std::time::Instant;
use tempfile::TempDir;

use super::corpus::{Corpus, PERFORMANCES_PER_ATHLETE};
use super::{measure, Phase};

const APPEND_BATCH: usize = 1_000;

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
            .map_or(0, |value| value);
        let rows = u64::try_from(rows).context("row count does not fit u64")?;
        anyhow::ensure!(
            found == rows,
            "{} holds {found} observations, expected {rows}",
            table.file()
        );
    }
    Ok(())
}

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

pub(super) fn load_phase(store: &Store, athletes: usize) -> Result<(ExportDataset, Phase)> {
    let started = Instant::now();
    let dataset = ExportDataset::load(store).context("loading the export dataset")?;
    let phase = measure("dataset", athletes, "athletes", started.elapsed())?;
    Ok((dataset, phase))
}

pub(super) fn census_phases(
    dataset: &ExportDataset,
    store: &Store,
    cohort: usize,
) -> Result<(Phase, Phase, workbook::Censuses)> {
    let started = Instant::now();
    let core = report::build_census(
        &Derivation::of(dataset, Scope::Core, None),
        &store.out_dir(),
    );
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
    let all_sources = report::build_census(
        &Derivation::of(dataset, Scope::AllSources, None),
        &store.out_dir(),
    );
    let all_sources_phase = measure("census_all_sources", cohort, "athletes", started.elapsed())?;
    anyhow::ensure!(
        all_sources.totals.athletes == cohort,
        "all-sources census counted {} athletes, expected {cohort}",
        all_sources.totals.athletes
    );
    Ok((
        core_phase,
        all_sources_phase,
        workbook::Censuses { core, all_sources },
    ))
}

pub(super) fn bests_phase(
    dataset: &ExportDataset,
    athletes: usize,
    performances: usize,
) -> Result<(Phase, usize)> {
    let started = Instant::now();
    let options = bests::Options {
        scope: Scope::AllSources,
        grad_year: Some(2027),
        limit: None,
    };
    let rows = bests::build_from_dataset(dataset, &options);
    let phase = measure("bests", performances, "performances", started.elapsed())?;
    anyhow::ensure!(
        rows.len() == athletes,
        "bests produced {} rows, expected {athletes}",
        rows.len()
    );
    for row in &rows {
        anyhow::ensure!(
            row.population.marks == PERFORMANCES_PER_ATHLETE,
            "athlete {} reduced {} marks, expected {}",
            row.athlete_id(),
            row.population.marks,
            PERFORMANCES_PER_ATHLETE
        );
    }
    Ok((phase, rows.len()))
}

pub(super) fn workbook_phase(
    dataset: &ExportDataset,
    store: &Store,
    dir: &TempDir,
    entities: usize,
    censuses: &workbook::Censuses,
) -> Result<Phase> {
    let out = dir.path().join("synthetic-census.xlsx");
    let started = Instant::now();
    let path = workbook::build_from(
        dataset,
        store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(out.clone()),
            limit: None,
            scope: Scope::AllSources,
            school_year: None,
        },
        censuses,
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
