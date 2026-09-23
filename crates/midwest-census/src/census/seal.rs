//! The seal: assembling §70's evidence, and the ladder it walks, wherever the caller holds the store.
//!
//! The seal is the one place the pipeline is allowed to call a census finished, and it may only
//! certify what it actually read. This module owns the assembly so both callers certify the same
//! thing by the same rules: the CLI, which opens the store in-process and can read no further than
//! the store, and the service, which also reads the run's own journal and passes those counts in.
//!
//! Nothing here is a claim by the caller. Every count is read from the store, the coverage
//! classifier or the workbook's own bytes, and the two journal counts are `Option`: `None` is
//! *unmeasured*, never zero, so a caller that could not read the journal cannot accidentally
//! certify the items the journal owns — it refuses instead, naming them.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use census_domain::model::{AccessBlockKind, ReviewCase, SourceAccessCondition};

use super::{
    owed_cohort_decisions, owed_identity_candidates, CensusState, GapTally, OpenWork,
    RetainedFindings, SealCounts, SealEvidence, SealedCensus, WorkbookCheck,
};
use crate::report::{self, Census, CoverageReport, Scope};
use crate::store::{Store, StoreStats, Table};

pub mod workbook;

mod ladder;

use ladder::{reached_phase, workbook_path};
use workbook::inspect_workbook;

/// The two §70 counts that only the run's own objects can answer.
///
/// They are separate from the store-side counts on purpose: a caller that holds no journal passes
/// `None` here and the seal says *unmeasured* rather than zero, which keeps the items those counts
/// belong to open instead of certifying them by default.
#[derive(Debug, Clone, Copy, Default)]
pub struct JournalCounts {
    /// Jurisdictions whose sweep still owes a stage.
    pub jurisdiction_sweeps: Option<u64>,
    /// Source objects that have accepted nothing yet.
    pub source_objects: Option<u64>,
}

/// What a seal run is asked to certify, and what its caller could read beyond the store.
#[derive(Debug, Clone)]
pub struct SealRequest {
    /// Graduation year of the cohort being certified.
    pub grad_year: i16,
    /// Which scope's coverage the seal reads.
    pub scope: Scope,
    /// The workbook to certify. `None` certifies the newest export in the store's out dir.
    pub workbook: Option<PathBuf>,
    /// Write the seal to `out/seal.json`, so a later run reads it instead of re-deriving it.
    pub write: bool,
    /// The run's own open work, when the caller could read the journal. `None` is unmeasured.
    pub journal: Option<JournalCounts>,
    /// Source failures the journal reported, when it reports them at all. `None` is unmeasured.
    pub source_failures: Option<u64>,
}

/// What the run found: the ladder, the evidence, and the seal or the refusal.
#[derive(Debug)]
pub struct SealOutcome {
    /// The ladder the store's own artifacts put this census on.
    pub state: CensusState,
    /// The seal a previous `--write` recorded: reported, never trusted.
    pub recorded: Option<SealedCensus>,
    /// The workbook this run certified.
    pub workbook: PathBuf,
    /// Every §70 item, measured from the evidence the run assembled.
    pub evidence: SealEvidence,
    /// The refusal, when the evidence did not hold, named by the item that blocked it.
    pub refusal: Option<String>,
    /// Where the seal was written, when the caller asked for that and the census sealed.
    pub wrote: Option<PathBuf>,
}

impl SealOutcome {
    /// The seal this run reached, when it reached one.
    pub fn sealed(&self) -> Option<&SealedCensus> {
        self.state.sealed()
    }
}

/// Assemble §70's evidence, walk the ladder, then seal the census or refuse by name.
///
/// Idempotent: the evidence is a pure function of the store, the workbook and the run's journal, so
/// running it twice over the same census seals the same census. A re-run writes the same bytes.
pub fn seal(store: &Store, request: &SealRequest) -> Result<SealOutcome> {
    let coverage = report::coverage_report(store, Some(request.grad_year))?;
    let census = report::build_census(store, request.scope)?;
    let stats = store.stats()?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    let access = store.scan::<SourceAccessCondition>(Table::SourceAccess)?;

    let Some(path) = workbook_path(store, request.workbook.as_deref())? else {
        bail!(
            "no workbook in {}: run `midwest-census workbook --grad-year {}` before sealing",
            store.out_dir().display(),
            request.grad_year
        );
    };
    let workbook = inspect_workbook(
        &path,
        count(census.totals.class_of_2027),
        count(coverage.jurisdictions.len()),
    )?;
    let evidence = assemble(&coverage, &census, &stats, &cases, &access, request, workbook);

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

/// Everything §70 asks the census to prove, from the store, the classifier and the run's journal.
fn assemble(
    coverage: &CoverageReport,
    census: &Census,
    stats: &StoreStats,
    cases: &[ReviewCase],
    access: &[SourceAccessCondition],
    request: &SealRequest,
    workbook: WorkbookCheck,
) -> SealEvidence {
    // Two counts come from the run's objects and two from the store's own rows, and they are not
    // interchangeable. A jurisdiction's stages and a source object's accepted observations live in
    // the workflow journal, which only the service can read; the cohort decisions and identity
    // candidates are retained review cases, which the store is the authority on. A count nobody
    // took is `None`, and `None` keeps its item open: that is what makes an offline seal refuse
    // rather than certify a completion it never checked.
    let journal = request.journal.unwrap_or_default();
    // The access conditions are read as rows, not as a ledger count: the split a reader needs is a
    // property of the rows' own `kind`, and one read answers both.
    let (access_conditions, blocked_hosts, throttled_hosts) = retained_access(access);
    SealEvidence {
        open: OpenWork {
            jurisdiction_sweeps: journal.jurisdiction_sweeps,
            source_objects: journal.source_objects,
            cohort_decisions: Some(owed_cohort_decisions(cases)),
            identity_candidates: Some(owed_identity_candidates(cases)),
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
            // §70 item 2 is about operations that stopped, and a *refused host* is the store's record
            // of that; a throttle is a slowdown with a cooldown, kept apart because reading it as a
            // stopped operation would overstate what the census lost.
            access_conditions,
            blocked_hosts,
            throttled_hosts,
            // A source *failure* is a terminal outcome of one attempt, and the store keeps access
            // conditions rather than attempts. The journal knows the objects that accepted nothing,
            // which is the other half of §70 item 2's fact; per-attempt failures are reported only where
            // the caller could read them.
            source_failures: request.source_failures,
            observations: stats.observations,
            calculations: count(coverage.read.performances),
        },
        workbook,
        observed_on: census.generated_on.clone(),
    }
}

/// The retained access conditions as the seal publishes them: the count, and the split a reader needs
/// to tell a refusal from a throttle.
///
/// Every other table's number comes from [`table_rows`], the store's own ledger. This one is read as
/// rows because the split is a property of the rows' `kind`, and the kinds are not one fact: a host
/// that refused (`Forbidden`, or a robots denial it did not authorize) ends the work it was asked for,
/// while a host that asked the lane to slow down (`RateLimited`, `Timeout`, `Unavailable`,
/// `BrowserUnavailable`) is bounded by a cooldown. `HumanRequired` sits with the refusals for the
/// reason its own row carries no cooldown: a profile that wants a person is not a slowdown, and
/// nothing but an operator ends it. A reader of `seal.json` should not have to open a second artifact
/// to tell those apart.
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
    // The remaining kinds are the slowdowns: the split is the store's own row count less the refusals,
    // so a kind that is neither cannot hide in a number that adds up to less than the table holds.
    let throttled = rows.len().saturating_sub(blocked);
    (count(rows.len()), count(blocked), count(throttled))
}

/// One table's row count as the store keeps it, per the table's own storage mode.
///
/// The two figures [`StoreStats`] carries are not one number under two names: a log's `tables` count is
/// the observations it has appended and its `appended` count is the next sequence it reserves from,
/// while a derived table's count is the rows its newest write left standing and its sequence pointer
/// never moves at all. `tables` is therefore the rows the store holds — the figure a seal certifies —
/// and it is a ledger rather than a walk of the keyspace: exact for what it counts, and cheap enough to
/// ask for every table on every seal. The walk that reads the keys themselves is [`Store::walk_table`],
/// and a ledger that disagrees with it is [`Store::integrity`]'s finding, not something a seal may
/// quietly walk past or silently repair.
///
/// A table the stats do not name reads as zero, which can only hold a seal back, never let one through.
fn table_rows(stats: &StoreStats, table: Table) -> u64 {
    stats
        .tables
        .iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, rows)| *rows)
        .unwrap_or(0)
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

/// Persist the sealed state, so a later run reads the seal instead of re-deriving it.
fn write_seal(store: &Store, state: &CensusState) -> Result<PathBuf> {
    let out = store.out_dir().join("seal.json");
    std::fs::write(&out, serde_json::to_vec_pretty(state)?)
        .with_context(|| format!("writing {}", out.display()))?;
    Ok(out)
}

/// `usize` counts become the `u64` the seal records. Saturating: a count that cannot be represented
/// is not a count this census may claim.
pub(crate) fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
