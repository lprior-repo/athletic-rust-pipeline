
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use census_domain::model::{AccessBlockKind, ReviewCase, SourceAccessCondition};

use super::{CensusState, SealEvidence, SealedCensus};
use census_report::report::{self, Scope};
use census_store::{Store, StoreStats, Table};

pub mod workbook;

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

pub fn seal(store: &Store, request: &SealRequest) -> Result<SealOutcome> {
    let coverage = report::coverage_report(store, Some(request.grad_year))?;
    let census = report::build_census(store, request.scope)?;
    let stats = store.stats()?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    let access = store.scan::<SourceAccessCondition>(Table::SourceAccess)?;

    let Some(path) = workbook_path(store, request.workbook.as_deref())? else {
        bail!(
            "no workbook in {}: run `census-service workbook --grad-year {}` before sealing",
            store.out_dir().display(),
            request.grad_year
        );
    };
    let workbook = inspect_workbook(&path, store, request.grad_year, request.scope)?;
    let evidence = assemble(
        &coverage, &census, &stats, &cases, &access, request, workbook,
    );

    let mut state = reached_phase(&stats, &path)?;
    let recorded = recorded_seal(store)?;
    let refusal = state
        .seal(evidence.clone())
        .err()
        .map(|blocked| blocked.to_string());
    let wrote = match (&refusal, request.write) {
        (None, true) => Some(write_seal(store, &state)?),
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
        .unwrap_or(0)
}

fn recorded_seal(store: &Store) -> Result<Option<SealedCensus>> {
    let path = store.out_dir().join("seal.json");
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let state: CensusState =
        serde_json::from_slice(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(state.sealed().cloned())
}

fn write_seal(store: &Store, state: &CensusState) -> Result<PathBuf> {
    let out = store.out_dir().join("seal.json");
    std::fs::write(&out, serde_json::to_vec_pretty(state)?)
        .with_context(|| format!("writing {}", out.display()))?;
    Ok(out)
}

pub(crate) fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
