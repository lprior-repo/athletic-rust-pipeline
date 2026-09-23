//! The mutable state one classification pass hands to the next: the per-jurisdiction accumulators,
//! the school sets the school and coach passes share, and the small primitives every pass needs.

use super::gaps::GapCounters;
use super::{CoverageGap, CoverageTotals, JurisdictionCoverage};
use census_domain::model::CanonicalAthlete;
use census_domain::model::CanonicalSchool;
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// One row's accumulating state: the published columns plus the gap counts no column carries.
#[derive(Debug, Default)]
pub(super) struct Bucket {
    pub(super) row: JurisdictionCoverage,
    pub(super) gaps: GapCounters,
}

impl Bucket {
    /// The finished row and its gap counts, with the two derived columns filled.
    pub(super) fn finish(
        mut self,
        bucket: JurisdictionBucket,
    ) -> (JurisdictionCoverage, GapCounters) {
        self.row.jurisdiction = bucket;
        self.row.core_share_pct = share_pct(self.row.athletes_core, self.row.athletes);
        (self.row, self.gaps)
    }
}

/// One accumulator per published row, keyed by the bucket it publishes in.
pub(super) type BucketMap = BTreeMap<JurisdictionBucket, Bucket>;

/// Per-athlete performance tallies: how many rows, how many carry a comparable mark, and how many
/// name an event the events table does not hold.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct PerfTally {
    pub(super) rows: usize,
    pub(super) comparable: usize,
    pub(super) missing_event_context: usize,
    pub(super) unmapped_event: usize,
}

/// The school ids the athlete and coach passes reached, counted once per school row each.
#[derive(Debug, Default)]
pub(super) struct SchoolSets<'a> {
    pub(super) with_athletes: BTreeSet<&'a str>,
    pub(super) with_tf_coach: BTreeSet<&'a str>,
    pub(super) with_xc_coach: BTreeSet<&'a str>,
    pub(super) with_coach_email: BTreeSet<&'a str>,
}

/// What one classification read and published, before the public report is composed from it.
pub(super) struct Outcome {
    pub(super) jurisdictions: Vec<JurisdictionCoverage>,
    pub(super) gaps: Vec<CoverageGap>,
    pub(super) read: CoverageTotals,
    /// What the census run scope left outside every published row: the read side's sibling count,
    /// which publishes as a provenance note instead of disappearing.
    pub(super) outside_scope: CoverageTotals,
    pub(super) off_cohort_athletes: usize,
}

/// The bucket for `bucket`, created on first use; every bucket is seeded, so creation is
/// unreachable.
pub(super) fn bucket_mut(buckets: &mut BucketMap, bucket: JurisdictionBucket) -> &mut Bucket {
    buckets.entry(bucket).or_default()
}

/// The jurisdiction one school id publishes in: its school row's state, else the unplaced row.
pub(crate) fn jurisdiction_of(
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    school: &str,
) -> JurisdictionBucket {
    school_state.get(school).copied().flatten().into()
}

/// School id to the jurisdiction of its school row, `None` when the row carries none. A school id
/// that is absent is a missing school, which the gap rows count separately.
pub(crate) fn school_state_index(
    schools: &[CanonicalSchool],
) -> HashMap<&str, Option<UsJurisdiction>> {
    schools
        .iter()
        .map(|school| (school.id.as_str(), school.state))
        .collect()
}

/// Whether one athlete is inside the requested cohort; `None` reads every stored athlete.
pub(super) fn in_cohort(athlete: &CanonicalAthlete, grad_year: Option<i16>) -> bool {
    match grad_year {
        Some(year) => athlete.grad_year.get() == year,
        None => true,
    }
}

/// A floored integer percentage of a whole, zero when the whole is zero.
pub(super) fn share_pct(part: usize, whole: usize) -> u32 {
    let part = u64::try_from(part).unwrap_or(u64::MAX);
    let whole = u64::try_from(whole).unwrap_or(u64::MAX);
    let percent = part
        .checked_mul(100)
        .and_then(|scaled| scaled.checked_div(whole));
    percent.map_or(0, |value| u32::try_from(value).unwrap_or(u32::MAX))
}
