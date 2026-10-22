use crate::bests;
use crate::report::{io_error, xlsx_error, Census, Derivation, ReportResult, Scope};
use census_domain::UsJurisdiction;
use census_store::Store;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod cells;
pub mod meta;
mod public_identity;
mod reported;

mod performances;
mod relays;
pub use meta::retained_records;
pub use performances::{PerformanceProjection, PerformanceRow, ProjectedValue};
mod recruiting;
pub use recruiting::{write_recruiting_csv, RecruitingCsvCounts};
mod options;
pub use options::{options_for_selected_dataset, options_for_selected_request, Options};
pub mod publication;
pub mod reference_comparison;
#[cfg(test)]
#[path = "publication/normalized/identity_aliases/tests/fixture.rs"]
pub(crate) mod selected_identity_fixture;
mod sheet_family;
#[cfg(test)]
mod tests;
pub mod verify;

pub struct Censuses {
    pub core: Census,
    pub all_sources: Census,
}

impl Censuses {
    pub fn of(
        dataset: &crate::export::ExportDataset,
        out: &Path,
        jurisdictions: &[UsJurisdiction],
    ) -> ReportResult<Self> {
        Ok(Self {
            core: scope_census(dataset, Scope::Core, out, jurisdictions)?,
            all_sources: scope_census(dataset, Scope::AllSources, out, jurisdictions)?,
        })
    }
}

fn scope_census(
    dataset: &crate::export::ExportDataset,
    scope: Scope,
    out: &Path,
    jurisdictions: &[UsJurisdiction],
) -> ReportResult<Census> {
    let derivation = Derivation::for_publication(dataset, scope, None, jurisdictions)?;
    Ok(crate::report::build_census(&derivation, out))
}

pub fn build(store: &Store, options: &Options) -> ReportResult<PathBuf> {
    let dataset = crate::export::ExportDataset::load(store)?;
    let censuses = Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    build_from(&dataset, store, options, &censuses)
}

pub fn build_from(
    dataset: &crate::export::ExportDataset,
    store: &Store,
    options: &Options,
    censuses: &Censuses,
) -> ReportResult<PathBuf> {
    publication::selected::ensure_current_options(dataset, options)?;
    crate::bests::participation::ParticipationSelection::of(dataset)?;
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
) -> ReportResult<publication::inventory::RowCounts> {
    let admission = crate::report::EventAdmission::of(dataset)?;
    let derivation = Derivation::for_publication(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    let outcome = select_bests(&derivation, &admission, options)?;
    let cohort = match options.grad_year.map(|year| format!("co{year}")) {
        Some(value) => value,
        None => "all".to_string(),
    };
    bests::write_outcome(directory, &derivation, &outcome, &cohort)?;
    let counts = publication::normalized::write_outcome(directory, &derivation, options, &outcome)?;
    dataset.save_frozen(&directory.join("frozen-input.json"))?;

    publication::write_json(&directory.join("event-catalog.json"), &admission.audit())?;
    render_artifacts(
        directory,
        dataset,
        options,
        censuses,
        outcome.selections,
        &admission,
        &derivation,
    )?;
    Ok(counts)
}

fn select_bests(
    derivation: &Derivation<'_>,
    admission: &crate::report::EventAdmission<'_>,
    options: &Options,
) -> ReportResult<bests::SelectionOutcome> {
    let started = Instant::now();
    let outcome = bests::build_from_derivation(derivation, admission, options.limit)?;
    tracing::info!(
        rows = outcome.selections.len(),
        ms = millis(started),
        "workbook build step: bests"
    );
    Ok(outcome)
}

fn render_artifacts(
    directory: &Path,
    dataset: &crate::export::ExportDataset,
    options: &Options,
    censuses: &Censuses,
    bests: Vec<bests::SharedSelection>,
    admission: &crate::report::EventAdmission<'_>,
    derivation: &Derivation<'_>,
) -> ReportResult<()> {
    let path = directory.join("workbook.xlsx");
    let started = Instant::now();
    let recruiting = recruiting::Recruiting::of(derivation, options.school_year, bests, admission)?;
    tracing::info!(ms = millis(started), "workbook build step: recruiting");
    let population =
        Derivation::for_publication(dataset, options.scope, None, &options.jurisdictions)?;
    let views = Views::of(
        censuses,
        &recruiting,
        derivation,
        &population,
        options.school_year,
        admission,
    );
    write_workbook(&path, views)?;
    relays::write_sidecars(directory, derivation, admission)?;
    publication::write_sidecars(
        directory,
        dataset,
        censuses,
        derivation,
        &population,
        options.school_year,
    )
}

#[derive(Clone, Copy)]
struct Views<'a> {
    core: &'a Census,
    all_sources: &'a Census,
    recruiting: &'a recruiting::Recruiting<'a>,
    derivation: &'a Derivation<'a>,
    population: &'a Derivation<'a>,
    school_year: census_domain::model::SchoolYear,
    admission: &'a crate::report::EventAdmission<'a>,
}

impl<'a> Views<'a> {
    fn of(
        censuses: &'a Censuses,
        recruiting: &'a recruiting::Recruiting<'a>,
        derivation: &'a Derivation<'a>,
        population: &'a Derivation<'a>,
        school_year: census_domain::model::SchoolYear,
        admission: &'a crate::report::EventAdmission<'a>,
    ) -> Self {
        Self {
            core: &censuses.core,
            all_sources: &censuses.all_sources,
            recruiting,
            derivation,
            population,
            school_year,
            admission,
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
    if views.derivation.dataset().selected_profile_id().is_some()
        && !views.derivation.dataset().is_historical_selected_replay()
    {
        views.recruiting.write_worksheet(book, path)?;
        views.recruiting.trace_counts();
        return Ok(());
    }
    write_recruiting_sheets(book, path, views.recruiting)?;
    write_performance_sheets(book, path, views.derivation)?;
    relays::write_sheet(book, path, views.derivation, views.admission)?;
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
            admission: views.admission,
        },
    )?;
    tracing::info!(ms = millis(started), "workbook build step: meta sheets");
    Ok(())
}

fn write_recruiting_sheets(
    book: &mut Workbook,
    path: &Path,
    recruiting: &recruiting::Recruiting<'_>,
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
    recruiting: &recruiting::Recruiting<'_>,
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
