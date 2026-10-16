use std::path::PathBuf;

use census_domain::UsJurisdiction;

use census_domain::model::{AccessBlockKind, CensusRun, GradYear, SourceAccessCondition};

use super::{CensusState, SealEvidence, SealedCensus, WorkbookCheck};
use census_report::export::ExportDataset;
use census_report::report::{self, CoverageReport, Derivation, Scope};
use census_store::{Store, StoreStats, Table};

pub mod workbook;

#[derive(Debug, thiserror::Error)]
pub enum SealWorkflowError {
    #[error("coverage report failed: {0}")]
    Coverage(String),
    #[error("store operation failed: {0}")]
    Store(String),
    #[error("workbook inspection failed: {0}")]
    Workbook(String),
    #[error("no workbook found; run census workbook before sealing")]
    WorkbookMissing,
    #[error("phase detection failed: {0}")]
    PhaseDetection(String),
    #[error("seal recording failed: {0}")]
    SealRecording(String),
    #[error("seal write failed: {0}")]
    SealWrite(String),
    #[error("the Class-of-2027 cohort is the only one that may be sealed; {requested} is not the census seal cohort")]
    ForeignCohort { requested: i16 },
    #[error("measured run {} does not match the store's bound run {bound}", run_display(.measured))]
    RunMismatch { measured: CensusRun, bound: String },
}

mod assembly;

mod ladder;

use assembly::assemble;
use ladder::{reached_phase, workbook_path};
use workbook::{inspect_workbook, Inspection};

#[derive(Debug, Clone, Default)]
pub struct JournalCounts {
    pub jurisdiction_sweeps: Option<u64>,
    pub source_objects: Option<u64>,
    pub silent_sources: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SealRequest {
    pub grad_year: i16,
    pub scope: Scope,
    pub jurisdictions: Vec<UsJurisdiction>,
    pub workbook: Option<PathBuf>,
    pub write: bool,
    pub journal: Option<JournalCounts>,
    pub source_failures: Option<u64>,
    pub run: Option<CensusRun>,
}

#[derive(Debug)]
pub struct SealOutcome {
    pub state: CensusState,
    pub recorded: Option<SealedCensus>,
    pub workbook: PathBuf,
    pub evidence: SealEvidence,
    pub refusal: Option<String>,
    pub wrote: Option<PathBuf>,
}

impl SealOutcome {
    pub fn sealed(&self) -> Option<&SealedCensus> {
        self.state.sealed()
    }
}

struct SealCoverage<'a> {
    selected: &'a CoverageReport,
    retained: &'a CoverageReport,
}

fn run_display(run: &CensusRun) -> String {
    format!("{}-{}", run.season().get(), run.revision())
}

fn bound_display(bound: Option<CensusRun>) -> String {
    match bound {
        Some(run) => run_display(&run),
        None => "absent".to_string(),
    }
}

pub fn seal(store: &Store, request: &SealRequest) -> Result<SealOutcome, SealWorkflowError> {
    let cohort = census_cohort(request.grad_year)?;
    bound_run(store, request.run)?;
    let (path, inspection) = publication(store, request, cohort)?;
    let (evidence, stats) = assemble_evidence(
        store,
        request,
        cohort,
        &inspection.dataset,
        inspection.check,
    )?;
    let recorded = recorded_seal(store)
        .map_err(|error| SealWorkflowError::SealRecording(error.to_string()))?;
    let source_fence = store.fenced_snapshot();
    inspection
        .dataset
        .ensure_snapshot(source_fence.view())
        .map_err(|error| SealWorkflowError::Workbook(error.to_string()))?;
    let mut state = reached_phase(&stats, &path, &evidence)
        .map_err(|error| SealWorkflowError::PhaseDetection(error.to_string()))?;
    let refusal = state
        .seal(evidence.clone())
        .err()
        .map(|blocked| blocked.to_string());
    let wrote = match (&refusal, request.write) {
        (None, true) => Some(
            write_seal(store, &state)
                .map_err(|error| SealWorkflowError::SealWrite(error.to_string()))?,
        ),
        _ => None,
    };
    Ok(SealOutcome {
        state,
        recorded,
        workbook: path,
        evidence,
        refusal,
        wrote,
    })
}

fn publication(
    store: &Store,
    request: &SealRequest,
    cohort: GradYear,
) -> Result<(PathBuf, Inspection), SealWorkflowError> {
    let Some(path) = workbook_path(store, request.workbook.as_deref())
        .map_err(|error| SealWorkflowError::Workbook(error.to_string()))?
    else {
        return Err(SealWorkflowError::WorkbookMissing);
    };
    let inspection = inspect_workbook(&path, store, cohort, request.scope, &request.jurisdictions)
        .map_err(|error| SealWorkflowError::Workbook(error.to_string()))?;
    Ok((path, inspection))
}

fn assemble_evidence(
    store: &Store,
    request: &SealRequest,
    cohort: GradYear,
    dataset: &ExportDataset,
    workbook: WorkbookCheck,
) -> Result<(SealEvidence, StoreStats), SealWorkflowError> {
    let derivation = Derivation::of(
        dataset,
        request.scope,
        Some(cohort.get()),
        &request.jurisdictions,
    )
    .map_err(|error| SealWorkflowError::Coverage(error.to_string()))?;
    let selected_coverage = report::coverage_report_for_derivation(&derivation)
        .map_err(|error| SealWorkflowError::Coverage(error.to_string()))?;
    let retained_coverage = report::coverage_report(dataset, Some(cohort.get()))
        .map_err(|error| SealWorkflowError::Coverage(error.to_string()))?;
    let census = report::build_census(&derivation, &store.out_dir());
    let stats = store
        .stats()
        .map_err(|error| SealWorkflowError::Store(error.to_string()))?;
    let evidence = assemble(
        SealCoverage {
            selected: &selected_coverage,
            retained: &retained_coverage,
        },
        &census,
        &stats,
        &dataset.review_cases,
        &dataset.source_access,
        request,
        workbook,
    );
    Ok((evidence, stats))
}

fn bound_run(store: &Store, measured: Option<CensusRun>) -> Result<(), SealWorkflowError> {
    let Some(measured) = measured else {
        return Ok(());
    };
    let bound = store
        .run_manifest()
        .map_err(|error| SealWorkflowError::Store(error.to_string()))?
        .map(|manifest| manifest.run);
    if bound == Some(measured) {
        return Ok(());
    }
    Err(SealWorkflowError::RunMismatch {
        measured,
        bound: bound_display(bound),
    })
}

pub fn census_cohort(grad_year: i16) -> Result<GradYear, SealWorkflowError> {
    match GradYear::new(grad_year) {
        Some(cohort) if cohort == GradYear::CO2027 => Ok(cohort),
        _ => Err(SealWorkflowError::ForeignCohort {
            requested: grad_year,
        }),
    }
}

fn retained_access(rows: &[SourceAccessCondition]) -> (u64, u64, u64) {
    let blocked = rows
        .iter()
        .filter(|row| matches!(row.kind, AccessBlockKind::Forbidden))
        .count();
    let throttled = rows.len().saturating_sub(blocked);
    (count(rows.len()), count(blocked), count(throttled))
}

fn table_rows(stats: &StoreStats, table: Table) -> u64 {
    stats
        .tables
        .iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, rows)| *rows)
        .map_or(0, |value| value)
}

fn recorded_seal(store: &Store) -> Result<Option<SealedCensus>, SealWorkflowError> {
    let path = store.out_dir().join("seal.json");
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read(&path)
        .map_err(|error| SealWorkflowError::SealRecording(error.to_string()))?;
    let state: CensusState = serde_json::from_slice(&raw)
        .map_err(|error| SealWorkflowError::SealRecording(error.to_string()))?;
    Ok(state.sealed().cloned())
}

fn write_seal(store: &Store, state: &CensusState) -> Result<PathBuf, SealWorkflowError> {
    let out = store.out_dir().join("seal.json");
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| SealWorkflowError::SealWrite(error.to_string()))?;
    census_store::read::publish_atomically(&out, |temporary| {
        std::fs::write(temporary, &bytes).map_err(|source| census_store::StoreError::Io {
            path: temporary.to_path_buf(),
            source,
        })
    })
    .map_err(|error| SealWorkflowError::SealWrite(error.to_string()))?;
    Ok(out)
}

pub(crate) fn count(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, |value| value)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod probe;
