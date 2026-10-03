use std::path::PathBuf;

use census_domain::model::{AccessBlockKind, SourceAccessCondition};

use super::{CensusState, SealEvidence, SealedCensus};
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation, Scope};
use census_store::{Store, StoreStats, Table};

pub mod workbook;

#[derive(Debug, thiserror::Error)]
pub enum SealWorkflowError {
    #[error("coverage report failed: {0}")]
    Coverage(String),
    #[error("census build failed: {0}")]
    CensusBuild(String),
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
}

mod assembly;

mod ladder;

use assembly::assemble;
use ladder::{reached_phase, workbook_path};
use workbook::inspect_workbook;

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
    pub workbook: Option<PathBuf>,
    pub write: bool,
    pub journal: Option<JournalCounts>,
    pub source_failures: Option<u64>,
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

pub fn seal(store: &Store, request: &SealRequest) -> Result<SealOutcome, SealWorkflowError> {
    let dataset = ExportDataset::load(store)
        .map_err(|error| SealWorkflowError::CensusBuild(error.to_string()))?;
    let coverage = report::coverage_report(&dataset, Some(request.grad_year))
        .map_err(|error| SealWorkflowError::Coverage(error.to_string()))?;
    let derivation = Derivation::of(&dataset, request.scope, Some(request.grad_year));
    let census = report::build_census(&derivation, &store.out_dir());
    let stats = store
        .stats()
        .map_err(|error| SealWorkflowError::Store(error.to_string()))?;
    let cases = &dataset.review_cases;
    let access = &dataset.source_access;

    let Some(path) = workbook_path(store, request.workbook.as_deref())
        .map_err(|error| SealWorkflowError::Workbook(error.to_string()))?
    else {
        return Err(SealWorkflowError::WorkbookMissing);
    };
    let workbook = inspect_workbook(&path, &dataset, request.grad_year, request.scope)
        .map_err(|error| SealWorkflowError::Workbook(error.to_string()))?;
    let evidence = assemble(&coverage, &census, &stats, cases, access, request, workbook);

    let mut state = reached_phase(&stats, &path)
        .map_err(|error| SealWorkflowError::PhaseDetection(error.to_string()))?;
    let recorded = recorded_seal(store)
        .map_err(|error| SealWorkflowError::SealRecording(error.to_string()))?;
    let source_fence = store.fenced_snapshot();
    dataset
        .ensure_snapshot(source_fence.view())
        .map_err(|error| SealWorkflowError::Workbook(error.to_string()))?;
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

fn retained_access(rows: &[SourceAccessCondition]) -> (u64, u64, u64) {
    let blocked = rows
        .iter()
        .filter(|row| {
            matches!(
                row.kind,
                AccessBlockKind::Forbidden
                    | AccessBlockKind::RobotsDisallowed
                    | AccessBlockKind::HumanRequired
            )
        })
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
