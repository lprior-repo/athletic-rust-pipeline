use crate::bests;
use crate::report::{io_error, xlsx_error, Census, Derivation, ReportResult, Scope};
use census_store::Store;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod cells;
pub mod meta;

pub use meta::retained_records;
mod recruiting;

#[derive(Debug, Clone)]
pub struct Options {
    pub grad_year: Option<i16>,
    pub out: Option<PathBuf>,
    pub limit: Option<usize>,
    pub scope: Scope,
    pub school_year: Option<census_domain::model::SchoolYear>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: Scope::AllSources,
            school_year: None,
        }
    }
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
    let generated_on = &dataset.lineage.generated_on;
    let school_year = options
        .school_year
        .or_else(|| census_domain::model::SchoolYear::from_date(generated_on))
        .ok_or_else(|| crate::report::ReportError::Invariant {
            detail: format!("cannot determine contact school year from {generated_on}"),
        })?;

    let started = Instant::now();
    let bests = bests::build_from_dataset(
        dataset,
        &bests::Options {
            scope: options.scope,
            grad_year: options.grad_year,
            limit: options.limit,
        },
    );
    tracing::info!(
        rows = bests.len(),
        ms = millis(started),
        "workbook build step: bests"
    );
    let cohort = options
        .grad_year
        .map(|year| format!("co{year}"))
        .unwrap_or_else(|| "all".to_string());
    bests::write(&store.out_dir(), &bests, &cohort)?;

    let path = options.out.clone().unwrap_or_else(|| {
        store
            .out_dir()
            .join(format!("census-service-{generated_on}.xlsx"))
    });

    let derivation = Derivation::of(dataset, options.scope, options.grad_year);
    let started = Instant::now();
    let recruiting = recruiting::Recruiting::of(&derivation, school_year, bests)?;
    tracing::info!(ms = millis(started), "workbook build step: recruiting");
    let population = Derivation::of(dataset, options.scope, None);
    let views = Views {
        core: &censuses.core,
        all_sources: &censuses.all_sources,
        recruiting: &recruiting,
        population: &population,
        school_year,
    };
    write_workbook(&path, store, views)?;
    Ok(path)
}

#[derive(Clone, Copy)]
struct Views<'a> {
    core: &'a Census,
    all_sources: &'a Census,
    recruiting: &'a recruiting::Recruiting,
    population: &'a Derivation<'a>,
    school_year: census_domain::model::SchoolYear,
}

fn write_workbook(path: &Path, store: &Store, views: Views<'_>) -> ReportResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    let mut book = Workbook::new();
    write_objective_sheets(&mut book, path, store, views)?;
    let started = Instant::now();
    book.save(path).map_err(|source| xlsx_error(path, source))?;
    tracing::info!(ms = millis(started), "workbook build step: save");
    Ok(())
}

fn write_objective_sheets(
    book: &mut Workbook,
    path: &Path,
    store: &Store,
    views: Views<'_>,
) -> ReportResult<()> {
    let recruiting = views.recruiting;
    let started = Instant::now();
    recruiting.write_athletes(book, path)?;
    tracing::info!(
        ms = millis(started),
        "workbook build step: recruiting athletes sheet"
    );
    let started = Instant::now();
    recruiting.write_coaches(book, path)?;
    tracing::info!(
        ms = millis(started),
        "workbook build step: recruiting coaches sheet"
    );
    recruiting.trace_counts();
    let started = Instant::now();
    meta::write_meta_sheets(
        book,
        path,
        meta::RunFacts {
            population: views.population,
            store,
            core: views.core,
            all_sources: views.all_sources,
            bests: recruiting.selected_prs(),
            school_year: views.school_year,
        },
    )?;
    tracing::info!(ms = millis(started), "workbook build step: meta sheets");
    Ok(())
}

pub(super) fn millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
