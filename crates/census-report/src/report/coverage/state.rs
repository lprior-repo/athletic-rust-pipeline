use super::gaps::GapCounters;
use super::{CoverageGap, CoverageTotals, JurisdictionCoverage};
use census_domain::model::{CanonicalAthlete, CanonicalSchool};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Default)]
pub(super) struct Bucket {
    pub(super) row: JurisdictionCoverage,
    pub(super) gaps: GapCounters,
}

impl Bucket {
    pub(super) fn finish(
        mut self,
        bucket: JurisdictionBucket,
    ) -> (JurisdictionCoverage, GapCounters) {
        self.row.jurisdiction = bucket;
        self.row.core_share_pct = share_pct(self.row.athletes_core, self.row.athletes);
        (self.row, self.gaps)
    }
}

pub(super) type BucketMap = BTreeMap<JurisdictionBucket, Bucket>;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct PerfTally {
    pub(super) rows: usize,
    pub(super) comparable: usize,
    pub(super) missing_event_context: usize,
    pub(super) unmapped_event: usize,
}

#[derive(Debug, Default)]
pub(super) struct SchoolSets<'a> {
    pub(super) with_athletes: BTreeSet<&'a str>,
    pub(super) with_tf_coach: BTreeSet<&'a str>,
    pub(super) with_xc_coach: BTreeSet<&'a str>,
    pub(super) with_coach_email: BTreeSet<&'a str>,
}

use super::ContactCounts;
use super::SourcePopulation;

pub(super) struct Outcome {
    pub(super) jurisdictions: Vec<JurisdictionCoverage>,

    pub(super) source_population: SourcePopulation,
    pub(super) contact: ContactCounts,
    pub(super) terminal_failures: BTreeMap<String, usize>,
    pub(super) raw_observations: usize,
    pub(super) scope_filtered: usize,
    pub(super) accepted_cohort: usize,
    pub(super) gaps: Vec<CoverageGap>,
    pub(super) read: CoverageTotals,
    pub(super) outside_scope: CoverageTotals,
    pub(super) off_cohort_athletes: usize,
}

pub(super) fn bucket_mut(buckets: &mut BucketMap, bucket: JurisdictionBucket) -> &mut Bucket {
    buckets.entry(bucket).or_default()
}

pub(crate) fn jurisdiction_of(
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    school: &str,
) -> JurisdictionBucket {
    school_state.get(school).copied().flatten().into()
}

pub(crate) fn school_state_index<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> HashMap<&'a str, Option<UsJurisdiction>> {
    schools
        .into_iter()
        .map(|school| (school.id.as_str(), school.state))
        .collect()
}

pub(crate) fn in_requested_year(athlete: &CanonicalAthlete, grad_year: Option<i16>) -> bool {
    grad_year.is_none_or(|year| athlete.grad_year.get() == year)
}

pub(super) fn share_pct(part: usize, whole: usize) -> Option<u32> {
    let part = u64::try_from(part).map_or(u64::MAX, std::convert::identity);
    let whole = u64::try_from(whole).map_or(u64::MAX, std::convert::identity);
    let percent = part
        .checked_mul(100)
        .and_then(|scaled| scaled.checked_div(whole))?;
    Some(u32::try_from(percent).map_or(u32::MAX, std::convert::identity))
}
