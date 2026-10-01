use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::{table_rows, CensusState};
use crate::census::Phase;
use census_store::{Store, StoreStats, Table};

pub(super) fn reached_phase(stats: &StoreStats, workbook: &Path) -> Result<CensusState> {
    let steps = [
        (Phase::Acquiring, stats.observations > 0),
        (Phase::Reconciling, table_rows(stats, Table::Snapshots) > 0),
        (
            Phase::Reviewing,
            table_rows(stats, Table::ReviewCases) > 0
                || table_rows(stats, Table::IdentityVerdicts) > 0,
        ),
        (Phase::ResolvingGaps, table_rows(stats, Table::Coverage) > 0),
        (Phase::Exporting, workbook.exists()),
    ];
    let mut state = CensusState::Discovering;
    for (phase, reached) in steps {
        if !reached {
            break;
        }
        state
            .advance(phase)
            .with_context(|| format!("walking the census ladder into {phase:?}"))?;
    }
    Ok(state)
}

pub(super) fn workbook_path(store: &Store, named: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(path) = named {
        if !path.exists() {
            bail!("{} does not exist", path.display());
        }
        return Ok(Some(path.to_path_buf()));
    }
    let root = store.out_dir().join("publication");
    let pointer = root.join("current");
    match std::fs::symlink_metadata(&pointer) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", pointer.display())),
        Ok(_) => census_report::workbook::publication::current_workbook(&root)
            .map(Some)
            .context("resolving the published generation"),
    }
}
