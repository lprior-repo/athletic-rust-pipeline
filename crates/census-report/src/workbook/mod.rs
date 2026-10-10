use crate::bests;
use crate::report::{io_error, xlsx_error, Census, Derivation, ReportResult, Scope};
use census_store::Store;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod cells;
pub mod meta;

mod performances;
pub use meta::retained_records;
pub use performances::{PerformanceProjection, PerformanceRow, ProjectedValue};
mod recruiting;
pub use recruiting::{write_recruiting_csv, RecruitingCsvCounts};
pub mod publication;
#[cfg(test)]
mod tests;
pub mod verify;

#[derive(Debug, Clone)]
pub struct Options {
    pub grad_year: Option<i16>,
    pub out: Option<PathBuf>,
    pub limit: Option<usize>,
    pub scope: Scope,
    pub school_year: census_domain::model::SchoolYear,
}

pub struct Censuses {
    pub core: Census,
    pub all_sources: Census,
}

impl Censuses {
    pub fn of(dataset: &crate::export::ExportDataset, out: &Path) -> Self {
        Self {
            core: scope_census(dataset, Scope::Core, out),
            all_sources: scope_census(dataset, Scope::AllSources, out),
        }
    }
}

fn scope_census(dataset: &crate::export::ExportDataset, scope: Scope, out: &Path) -> Census {
    crate::report::build_census(&crate::report::Derivation::of(dataset, scope, None), out)
}

pub fn build(store: &Store, options: &Options) -> ReportResult<PathBuf> {
    let dataset = crate::export::ExportDataset::load(store)?;
    let censuses = Censuses::of(&dataset, &store.out_dir());
    build_from(&dataset, store, options, &censuses)
}

pub fn build_from(
    dataset: &crate::export::ExportDataset,
    store: &Store,
    options: &Options,
    censuses: &Censuses,
) -> ReportResult<PathBuf> {
    let stage = publication::Stage::begin(dataset, store, options)?;
    stage.publish(dataset, options, |directory| {
        write_artifacts(directory, dataset, options, censuses)
    })
}

fn write_artifacts(
    directory: &Path,
    dataset: &crate::export::ExportDataset,
    options: &Options,
    censuses: &Censuses,
) -> ReportResult<()> {
    let bests = select_bests(dataset, options);
    let cohort = match options.grad_year.map(|year| format!("co{year}")) {
        Some(value) => value,
        None => "all".to_string(),
    };
    bests::write(directory, &bests, &cohort)?;
    dataset.save_frozen(&directory.join("frozen-input.json"))?;

    render_artifacts(directory, dataset, options, censuses, bests)
}

fn select_bests(
    dataset: &crate::export::ExportDataset,
    options: &Options,
) -> Vec<bests::SharedSelection> {
    let started = Instant::now();
    let rows = bests::build_from_dataset(
        dataset,
        &bests::Options {
            scope: options.scope,
            grad_year: options.grad_year,
            limit: options.limit,
        },
    );
    tracing::info!(
        rows = rows.len(),
        ms = millis(started),
        "workbook build step: bests"
    );
    rows
}

fn render_artifacts(
    directory: &Path,
    dataset: &crate::export::ExportDataset,
    options: &Options,
    censuses: &Censuses,
    bests: Vec<bests::SharedSelection>,
) -> ReportResult<()> {
    let path = directory.join("workbook.xlsx");
    let derivation = Derivation::of(dataset, options.scope, options.grad_year);
    let started = Instant::now();
    let recruiting = recruiting::Recruiting::of(&derivation, options.school_year, bests)?;
    tracing::info!(ms = millis(started), "workbook build step: recruiting");
    let population = Derivation::of(dataset, options.scope, None);
    let views = Views::of(
        censuses,
        &recruiting,
        &derivation,
        &population,
        options.school_year,
    );
    write_workbook(&path, views)?;
    publication::write_sidecars(
        directory,
        dataset,
        censuses,
        &derivation,
        &population,
        options.school_year,
    )
}

#[derive(Clone, Copy)]
struct Views<'a> {
    core: &'a Census,
    all_sources: &'a Census,
    recruiting: &'a recruiting::Recruiting,
    derivation: &'a Derivation<'a>,
    population: &'a Derivation<'a>,
    school_year: census_domain::model::SchoolYear,
}

impl<'a> Views<'a> {
    fn of(
        censuses: &'a Censuses,
        recruiting: &'a recruiting::Recruiting,
        derivation: &'a Derivation<'a>,
        population: &'a Derivation<'a>,
        school_year: census_domain::model::SchoolYear,
    ) -> Self {
        Self {
            core: &censuses.core,
            all_sources: &censuses.all_sources,
            recruiting,
            derivation,
            population,
            school_year,
        }
    }
}

fn write_workbook(path: &Path, views: Views<'_>) -> ReportResult<()> {
    if let Some(parent) = path.parent() {
        census_store::fs::create_dir_all_synced(parent)
            .map_err(|source| io_error(parent, source))?;
    }
    let mut book = Workbook::new();
    write_objective_sheets(&mut book, path, views)?;
    let started = Instant::now();
    book.save(path).map_err(|source| xlsx_error(path, source))?;
    tracing::info!(ms = millis(started), "workbook build step: save");
    Ok(())
}

fn write_objective_sheets(book: &mut Workbook, path: &Path, views: Views<'_>) -> ReportResult<()> {
    write_recruiting_sheets(book, path, views.recruiting)?;
    write_performance_sheets(book, path, views.derivation)?;
    write_coach_sheet(book, path, views.recruiting)?;
    views.recruiting.trace_counts();
    let started = Instant::now();
    meta::write_meta_sheets(
        book,
        path,
        meta::RunFacts {
            population: views.population,
            recruiting: views.derivation,
            core: views.core,
            all_sources: views.all_sources,
            bests: views.recruiting.selected_prs(),
            school_year: views.school_year,
        },
    )?;
    tracing::info!(ms = millis(started), "workbook build step: meta sheets");
    Ok(())
}

fn write_recruiting_sheets(
    book: &mut Workbook,
    path: &Path,
    recruiting: &recruiting::Recruiting,
) -> ReportResult<()> {
    let started = Instant::now();
    recruiting.write_athletes(book, path)?;
    tracing::info!(
        ms = millis(started),
        "workbook build step: recruiting athletes sheet"
    );
    let started = Instant::now();
    recruiting.write_prs(book, path)?;
    tracing::info!(
        ms = millis(started),
        "workbook build step: recruiting prs sheet"
    );
    Ok(())
}

fn write_performance_sheets(
    book: &mut Workbook,
    path: &Path,
    derivation: &Derivation<'_>,
) -> ReportResult<()> {
    let started = Instant::now();
    performances::write_performance_sheets(book, path, derivation)?;
    tracing::info!(
        ms = millis(started),
        "workbook build step: performances sheets"
    );
    Ok(())
}

fn write_coach_sheet(
    book: &mut Workbook,
    path: &Path,
    recruiting: &recruiting::Recruiting,
) -> ReportResult<()> {
    let started = Instant::now();
    recruiting.write_coaches(book, path)?;
    tracing::info!(
        ms = millis(started),
        "workbook build step: recruiting coaches sheet"
    );
    Ok(())
}

pub(super) fn millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).map_or(u64::MAX, |value| value)
}
