use crate::bests::{self, SharedSelection};
use crate::report::{build_census, io_error, xlsx_error, Census, ReportResult, Scope};
use census_store::Store;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};

mod cells;
pub mod meta;

pub use meta::retained_records;
mod performances;
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

pub fn build(store: &Store, options: &Options) -> ReportResult<PathBuf> {
    let core = build_census(store, Scope::Core)?;
    let all_sources = build_census(store, Scope::AllSources)?;
    let school_year = options.school_year
        .or_else(|| census_domain::model::SchoolYear::from_date(&core.generated_on))
        .ok_or_else(|| crate::report::ReportError::Invariant {
            detail: format!("cannot determine contact school year from {}", core.generated_on),
        })?;
    let bests = bests::build(
        store,
        &bests::Options {
            scope: options.scope,
            grad_year: options.grad_year,
            limit: options.limit,
        },
    )?;
    let cohort = options
        .grad_year
        .map(|year| format!("co{year}"))
        .unwrap_or_else(|| "all".to_string());
    bests::write(&store.out_dir(), &bests, &cohort)?;

    let path = options.out.clone().unwrap_or_else(|| {
        store
            .out_dir()
            .join(format!("census-service-{}.xlsx", core.generated_on))
    });
    write_workbook(
        &path,
        store,
        &core,
        &all_sources,
        bests,
        options,
        school_year,
    )?;
    Ok(path)
}

#[derive(Clone, Copy)]
struct Views<'a> {
    core: &'a Census,
    all_sources: &'a Census,
    recruiting: &'a recruiting::Recruiting,
    school_year: census_domain::model::SchoolYear,
}

fn write_workbook(
    path: &Path,
    store: &Store,
    core: &Census,
    all_sources: &Census,
    bests: Vec<SharedSelection>,
    options: &Options,
    school_year: census_domain::model::SchoolYear,
) -> ReportResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    let mut book = Workbook::new();
    let recruiting = recruiting::Recruiting::load(
        store, options.scope, options.grad_year, school_year, bests,
    )?;
    let views = Views {
        core,
        all_sources,
        recruiting: &recruiting,
        school_year,
    };
    write_objective_sheets(&mut book, path, store, views, options.scope, options.grad_year)?;
    book.save(path).map_err(|source| xlsx_error(path, source))?;
    Ok(())
}

fn write_objective_sheets(
    book: &mut Workbook,
    path: &Path,
    store: &Store,
    views: Views<'_>,
    scope: Scope,
    grad_year: Option<i16>,
) -> ReportResult<()> {
    let recruiting = views.recruiting;
    recruiting.write_athletes(book, path)?;
    recruiting.write_prs(book, path)?;
    let perf_population =
        performances::write_performance_sheets(book, path, store, scope, grad_year)?;
    let perf_population = performances::PerformanceSheetPopulation {
        cohort_year: perf_population.cohort_year,
        scope: perf_population.scope,
        cohort_athletes: recruiting.cohort_athletes(),
        total_rows: perf_population.total_rows,
    };
    recruiting.write_coaches(book, path)?;
    recruiting.trace_counts();
    meta::write_meta_sheets(
        book,
        path,
        meta::RunFacts {
            store,
            core: views.core,
            all_sources: views.all_sources,
            bests: recruiting.selected_prs(),
            scope,
            school_year: views.school_year,
            perf_population,
        },
    )
}
