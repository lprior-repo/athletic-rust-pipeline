//! The ladder a seal walks, and the workbook it certifies.
//!
//! Both answers come from the store's own artifacts rather than from a caller: [`reached_phase`]
//! enters a phase only when the artifact that phase produces exists, and [`workbook_path`] resolves
//! which export that artifact is. They are split out of `seal.rs` for the §38 line budget; nothing
//! about either rule lives here that `seal.rs`'s assembly needs to be read beside.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::{table_rows, CensusState};
use crate::census::Phase;
use census_store::{Store, StoreStats, Table};

/// Which phase the store's own artifacts put this census in.
///
/// Each step is entered only when the artifact that phase produces exists, so the ladder a seal
/// walks is the pipeline's recorded progress and not a caller's claim. The conditions are monotone:
/// a later artifact cannot exist without the earlier ones, so the walk cannot skip a phase.
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

/// The workbook this run certifies: the named path, or the newest export in the store's out dir.
pub(super) fn workbook_path(store: &Store, named: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(path) = named {
        if !path.exists() {
            bail!("{} does not exist", path.display());
        }
        return Ok(Some(path.to_path_buf()));
    }
    let out = store.out_dir();
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&out).with_context(|| format!("reading {}", out.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "xlsx") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if newest.as_ref().is_none_or(|(seen, _)| modified > *seen) {
            newest = Some((modified, path));
        }
    }
    Ok(newest.map(|(_, path)| path))
}
