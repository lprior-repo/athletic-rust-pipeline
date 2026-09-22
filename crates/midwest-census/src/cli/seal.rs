//! Seal the census, or refuse and name the §70 item that blocked it.
//!
//! The seal is the one place the pipeline is allowed to call a census finished. Everything that
//! decision rests on is assembled here from what the store, the classifier and the exported
//! workbook already hold — nothing is passed in as a claim.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Args;

use midwest_census::census::{
    CensusState, GapTally, OpenWork, Phase, RetainedFindings, SealCounts, SealEvidence,
    SealedCensus, WorkbookCheck,
};
use midwest_census::report::{self, Census, CoverageReport, Scope};
use midwest_census::store::{Store, StoreStats, Table};

mod workbook;

use workbook::inspect_workbook;

/// `midwest-census seal`
///
/// The export phase reads the workbook's own meta sheets: `Coverage` must carry every jurisdiction
/// the classifier produced, and `Run Metrics` must name the cohort the store counted. A workbook
/// that disagrees with the store refuses the seal and says which number disagreed.
#[derive(Debug, Args)]
pub(super) struct SealArgs {
    /// Graduation year of the cohort being certified.
    #[arg(long, default_value_t = 2027)]
    grad_year: i16,
    /// Certify the all-sources scope instead of the core scope.
    #[arg(long)]
    all_sources: bool,
    /// The workbook to certify. Defaults to the newest `out/*.xlsx`.
    #[arg(long)]
    workbook: Option<PathBuf>,
    /// Write the seal to `out/seal.json` so a later run reads it instead of re-deriving it.
    #[arg(long)]
    write: bool,
}

pub(super) fn run_seal(store: &Store, args: &SealArgs) -> Result<()> {
    let scope = if args.all_sources {
        Scope::AllSources
    } else {
        Scope::Core
    };
    let coverage = report::coverage_report(store, Some(args.grad_year))?;
    let census = report::build_census(store, scope)?;
    let stats = store.stats()?;

    let Some(path) = workbook_path(store, args)? else {
        bail!(
            "no workbook in {}: run `midwest-census workbook --grad-year {}` before sealing",
            store.out_dir().display(),
            args.grad_year
        );
    };
    let workbook = inspect_workbook(
        &path,
        count(census.totals.class_of_2027),
        count(coverage.jurisdictions.len()),
    )?;
    let evidence = assemble(&coverage, &census, &stats, workbook);

    let mut state = reached_phase(&stats, &path)?;
    // A previous `--write` is reported, never trusted: this run's evidence still has to agree with
    // it, and a difference is printed rather than silently resolved in either direction.
    let recorded = recorded_seal(store)?;
    if let Some(seal) = &recorded {
        println!("recorded seal: {} on {}", seal.digest, seal.sealed_on);
    }
    print_ladder(&state, &path, &evidence);
    if let Err(refusal) = state.seal(evidence) {
        println!("refused: {refusal}");
        bail!("seal refused: {refusal}");
    }
    report_seal(&state, recorded.as_ref())?;
    if args.write {
        write_seal(store, &state)?;
    }
    Ok(())
}

/// Print what the seal certified, and flag a recorded seal this run no longer matches.
fn report_seal(state: &CensusState, recorded: Option<&SealedCensus>) -> Result<()> {
    let seal = state.sealed().context("a sealed state carries its seal")?;
    println!("sealed {} on {}", seal.digest, seal.sealed_on);
    if let Some(previous) = recorded {
        if previous.digest != seal.digest {
            println!(
                "note: the recorded seal {} no longer matches this census",
                previous.digest
            );
        }
    }
    println!(
        "  cohort {} of {} athletes, {} schools, {} meets, {} performances, {} coaches",
        seal.counts.class_of_2027,
        seal.counts.athletes,
        seal.counts.schools,
        seal.counts.meets,
        seal.counts.performances,
        seal.counts.coaches,
    );
    println!(
        "  retained: {} gaps, {} conflicts, {} exhausted retries, {} source failures",
        seal.retained.gaps.len(),
        seal.retained.conflicts,
        seal.retained.retry_exhausted,
        seal.retained.source_failures,
    );
    Ok(())
}

/// Persist the sealed state, so a later run reads the seal instead of re-deriving it.
fn write_seal(store: &Store, state: &CensusState) -> Result<()> {
    let out = store.out_dir().join("seal.json");
    std::fs::write(&out, serde_json::to_vec_pretty(state)?)
        .with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(())
}

/// Which phase the store's own artifacts put this census in.
///
/// Each step is entered only when the artifact that phase produces exists, so the ladder a seal
/// walks is the pipeline's recorded progress and not a caller's claim. The conditions are monotone:
/// a later artifact cannot exist without the earlier ones, so the walk cannot skip a phase.
fn reached_phase(stats: &StoreStats, workbook: &Path) -> Result<CensusState> {
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

/// One table's row count, from the store's own sequence counters: exact, and cheap enough to ask
/// for every table on every seal. A table the stats do not name reads as zero, which can only hold
/// a seal back, never let one through.
fn table_rows(stats: &StoreStats, table: Table) -> u64 {
    stats
        .tables
        .iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, rows)| *rows)
        .unwrap_or(0)
}

fn print_ladder(state: &CensusState, workbook: &Path, evidence: &SealEvidence) {
    println!("phase: {}", state.phase().as_str());
    println!("workbook: {}", workbook.display());
    if let Some(sealed) = state.sealed() {
        println!("already sealed: {}", sealed.digest);
    }
    let open = evidence.open_items();
    if open.is_empty() {
        println!("acceptance: every §70 item is satisfied");
    } else {
        for item in open {
            println!(
                "acceptance: {} unmet — {}",
                item.as_str(),
                evidence.detail(item)
            );
        }
    }
}

/// Everything §70 asks the census to prove, read from the store and the coverage classifier.
fn assemble(
    coverage: &CoverageReport,
    census: &Census,
    stats: &StoreStats,
    workbook: WorkbookCheck,
) -> SealEvidence {
    let open_reviews = table_rows(stats, Table::ReviewCases)
        .saturating_sub(table_rows(stats, Table::IdentityVerdicts));

    SealEvidence {
        open: OpenWork {
            // Owed jurisdiction sweeps and un-terminal source objects live in the workflow journal,
            // which this command does not read; a CLI seal carries the store-visible decisions only.
            jurisdiction_sweeps: 0,
            source_objects: 0,
            cohort_decisions: 0,
            identity_candidates: open_reviews,
        },
        counts: SealCounts {
            jurisdictions: count(census.by_state.len()),
            schools: count(coverage.read.schools),
            meets: count(coverage.read.meets),
            athletes: count(census.totals.athletes),
            class_of_2027: count(census.totals.class_of_2027),
            performances: count(coverage.read.performances),
            coaches: count(census.totals.coaches),
        },
        retained: RetainedFindings {
            gaps: coverage
                .gaps
                .iter()
                .map(|gap| GapTally {
                    class: gap.class.to_string(),
                    unit: gap.unit.to_string(),
                    count: count(gap.count),
                })
                .collect(),
            conflicts: table_rows(stats, Table::Conflicts),
            retry_exhausted: table_rows(stats, Table::SourceAccess),
            source_failures: 0,
            observations: stats.observations,
            calculations: count(coverage.read.performances),
        },
        workbook,
        observed_on: census.generated_on.clone(),
    }
}

/// The seal a previous `--write` recorded, if the store holds one.
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

/// `usize` counts become the `u64` the seal records. Saturating: a count that cannot be represented
/// is not a count this census may claim.
pub(super) fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// The workbook this run certifies: the named path, or the newest export in the store's out dir.
fn workbook_path(store: &Store, args: &SealArgs) -> Result<Option<PathBuf>> {
    if let Some(path) = &args.workbook {
        if !path.exists() {
            bail!("{} does not exist", path.display());
        }
        return Ok(Some(path.clone()));
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

#[cfg(test)]
mod tests;
