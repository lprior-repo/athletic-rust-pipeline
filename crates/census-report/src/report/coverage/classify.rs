//! The store reads and the rows they classify: seeding, schools, coaches, meets, the core count and
//! publication. Each pass fills one row's columns from one table, so the whole report is seven scans
//! of already-merged tables and no derived cache. The other half of the contract — what those scans
//! held for the rows this report publishes — is counted beside the passes in [`super::reads`].

use super::super::{retain_core, ReportResult};
use super::gaps;
use super::reads::{self, Published, Tables};
use super::state::{
    bucket_mut, in_cohort, jurisdiction_of, school_state_index, Bucket, BucketMap, Outcome,
    SchoolSets,
};
use super::{athletes, CoverageGap, JurisdictionCoverage};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CoachRole, EventKind, Sport,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use census_store::{Store, Table};
use std::collections::{HashMap, HashSet};

/// The merged tables one coverage pass reads, plus the event ids its performance tallies join to.
/// Scanned once so the passes and the read side describe the same store.
struct Scanned {
    schools: Vec<CanonicalSchool>,
    coaches: Vec<CanonicalCoach>,
    meets: Vec<CanonicalMeet>,
    athletes: Vec<CanonicalAthlete>,
    performances: Vec<CanonicalPerformance>,
    event_ids: HashSet<String>,
    /// The subset of `event_ids` whose kind is `Unmapped`; the vocabulary gaps, keyed by id so the
    /// performance pass joins to them the way it joins to the events table itself.
    unmapped_event_ids: HashSet<String>,
}

impl Scanned {
    /// Scan every merged table the report reads.
    fn read(store: &Store) -> ReportResult<Self> {
        let events = store.scan::<CanonicalEvent>(Table::Events)?;
        let event_ids = events
            .iter()
            .map(|event| event.id.as_str().to_string())
            .collect();
        let unmapped_event_ids = events
            .iter()
            .filter(|event| matches!(event.kind, EventKind::Unmapped { .. }))
            .map(|event| event.id.as_str().to_string())
            .collect();
        Ok(Self {
            schools: store.scan(Table::Schools)?,
            coaches: store.scan(Table::Coaches)?,
            meets: store.scan(Table::Meets)?,
            athletes: store.scan(Table::Athletes)?,
            performances: store.scan(Table::Performances)?,
            event_ids,
            unmapped_event_ids,
        })
    }

    /// The read side's view of this scan: the entities, without the event ids it never joins to.
    fn tables(&self) -> Tables<'_> {
        Tables {
            schools: &self.schools,
            athletes: &self.athletes,
            coaches: &self.coaches,
            meets: &self.meets,
            performances: &self.performances,
        }
    }
}

/// §49 coverage for the store's merged tables. See [`super::coverage_report`] for the contract this
/// fills: the cohort filter it applies to the athlete-derived columns, and the run-scope universe
/// ([`Published`]) every row and every read counter is scoped by.
pub(super) fn run(store: &Store, grad_year: Option<i16>) -> ReportResult<Outcome> {
    let mut scanned = Scanned::read(store)?;
    let school_state = school_state_index(&scanned.schools);
    let universe = Published::new(jurisdiction_buckets());
    // The read side is counted from the tables before a column is filled, so no pass below can move
    // it: a row a pass loses or invents stays visible to the reconciliation.
    let reads = reads::totals(&scanned.tables(), &school_state, &universe, grad_year);

    let coach_schools: HashSet<&str> = scanned
        .coaches
        .iter()
        .map(|coach| coach.school.as_str())
        .collect();
    let mut sets = SchoolSets::default();
    let mut buckets = seed_buckets();
    let off_cohort_athletes = athletes::classify(
        &scanned.athletes,
        &school_state,
        &coach_schools,
        grad_year,
        &mut buckets,
        &mut sets.with_athletes,
    );
    let athlete_ids: HashSet<&str> = scanned.athletes.iter().map(|a| a.id.as_str()).collect();
    let (perf_tallies, orphan_performances) = athletes::tally_performances(
        &scanned.performances,
        &athlete_ids,
        &scanned.event_ids,
        &scanned.unmapped_event_ids,
    );
    athletes::classify_performances(
        &scanned.athletes,
        &school_state,
        &perf_tallies,
        &orphan_performances,
        grad_year,
        &mut buckets,
    );
    // Coaches before schools: the school pass counts the sets the coach pass fills.
    classify_coaches(&scanned.coaches, &school_state, &mut buckets, &mut sets);
    classify_schools(&scanned.schools, &sets, &mut buckets);
    classify_meets(&scanned.meets, &mut buckets);
    // Last: `retain_core` deletes exactly the non-core evidence every column above counted.
    count_core(
        &mut scanned.athletes,
        &school_state,
        grad_year,
        &mut buckets,
    );
    let (jurisdictions, gaps) = publish(buckets);

    Ok(Outcome {
        jurisdictions,
        gaps,
        read: reads.published,
        outside_scope: reads.outside,
        off_cohort_athletes,
    })
}

/// One accumulator per configured jurisdiction plus the unplaceable row, so no pass ever creates a
/// row the publication order forgot.
fn seed_buckets() -> BucketMap {
    let mut buckets = BucketMap::new();
    for bucket in jurisdiction_buckets() {
        buckets.insert(bucket, Bucket::default());
    }
    buckets
}

/// Every published row's bucket, in publication order: [`UsJurisdiction::CENSUS_SCOPE`], then the
/// unplaced row.
///
/// This one iterator defines the report's universe: the rows come from it, and [`Published::new`]
/// builds the read side's scope from it, so a read counter and a published row cannot disagree about
/// which jurisdictions exist.
fn jurisdiction_buckets() -> impl Iterator<Item = JurisdictionBucket> {
    UsJurisdiction::CENSUS_SCOPE
        .iter()
        .copied()
        .map(JurisdictionBucket::from)
        .chain(std::iter::once(JurisdictionBucket::Unplaced))
}

/// The school pass: the universe, plus the columns that count schools the other passes reached.
fn classify_schools(schools: &[CanonicalSchool], sets: &SchoolSets<'_>, buckets: &mut BucketMap) {
    for school in schools {
        let bucket = bucket_mut(buckets, JurisdictionBucket::from(school.state));
        bump(&mut bucket.row.schools);
        let id = school.id.as_str();
        if sets.with_athletes.contains(id) {
            bump(&mut bucket.row.schools_with_athletes);
        }
        if sets.with_tf_coach.contains(id) {
            bump(&mut bucket.row.schools_with_tf_coach);
        }
        if sets.with_xc_coach.contains(id) {
            bump(&mut bucket.row.schools_with_xc_coach);
        }
        if sets.with_coach_email.contains(id) {
            bump(&mut bucket.row.schools_with_coach_email);
        }
    }
}

/// The coach pass: the coach columns, plus the school sets the school pass counts. A head coach with
/// no sport is a school-wide row, and `school_coach_index` does not treat one as a track coach, so
/// neither does this pass: only `Some(sport)` sets the TF or XC column.
fn classify_coaches<'a>(
    coaches: &'a [CanonicalCoach],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    buckets: &mut BucketMap,
    sets: &mut SchoolSets<'a>,
) {
    for coach in coaches {
        let bucket = bucket_mut(
            buckets,
            jurisdiction_of(school_state, coach.school.as_str()),
        );
        bump(&mut bucket.row.coaches);
        let school = coach.school.as_str();
        if coach.professional_email.is_some() {
            bump(&mut bucket.row.coaches_with_email);
            sets.with_coach_email.insert(school);
        }
        if coach.role != CoachRole::HeadCoach {
            continue;
        }
        match coach.sport {
            Some(Sport::OutdoorTrack | Sport::IndoorTrack) => {
                sets.with_tf_coach.insert(school);
            }
            Some(Sport::CrossCountry) => {
                sets.with_xc_coach.insert(school);
            }
            None => {}
        }
    }
}

/// The meet pass: the meet table's own jurisdictions. Meets are not cohort-scoped, so the cohort
/// filter never narrows them.
fn classify_meets(meets: &[CanonicalMeet], buckets: &mut BucketMap) {
    for meet in meets {
        bump(
            &mut bucket_mut(buckets, JurisdictionBucket::from(meet.state))
                .row
                .meets,
        );
    }
}

/// The core column, counted after every all-sources column is filled: `athletes_core` is a sub-column
/// of `athletes`, so `retain_core` narrows it and never the totals the reconciliation compares.
fn count_core(
    athletes: &mut Vec<CanonicalAthlete>,
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    grad_year: Option<i16>,
    buckets: &mut BucketMap,
) {
    retain_core(athletes);
    for athlete in athletes.iter() {
        if !in_cohort(athlete, grad_year) {
            continue;
        }
        let bucket = jurisdiction_of(school_state, athlete.school.as_str());
        bump(&mut bucket_mut(buckets, bucket).row.athletes_core);
    }
}

/// The published rows in [`UsJurisdiction::CENSUS_SCOPE`] order with the unplaceable row last, plus
/// the gap rows each one produced.
///
/// An accumulator outside that universe — a mirror's Alaska or Hawaii rows, which no run covers
/// (ADR-009) — is dropped here rather than published, and [`super::reads`] counts it on the read
/// side's outside split so the drop is recorded instead of silent.
fn publish(mut buckets: BucketMap) -> (Vec<JurisdictionCoverage>, Vec<CoverageGap>) {
    let mut jurisdictions =
        Vec::with_capacity(UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    let mut gap_rows = Vec::new();
    for bucket in jurisdiction_buckets() {
        // A missing accumulator still publishes a row of zeros: an omitted jurisdiction is the one
        // outcome a coverage report may not produce.
        let (row, counters) = buckets.remove(&bucket).unwrap_or_default().finish(bucket);
        gap_rows.extend(gaps::rows(&row, &counters));
        jurisdictions.push(row);
    }
    (jurisdictions, gap_rows)
}

/// Saturating counter bump. The crate's report helpers are `pub(super)` to `report`, so this part
/// keeps its own copy rather than widening their visibility.
fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}
