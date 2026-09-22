//! The store reads and the rows they classify: seeding, schools, coaches, meets, the core count and
//! publication. Each pass fills one row's columns from one table, so the whole report is seven scans
//! of already-merged tables and no derived cache.

use super::super::{retain_core, ReportResult};
use super::gaps;
use super::state::{
    bucket_mut, in_cohort, jurisdiction_of, school_state_index, Bucket, BucketMap, Outcome,
    SchoolSets,
};
use super::{athletes, CoverageGap, CoverageTotals, JurisdictionCoverage};
use crate::store::{Store, Table};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CoachRole, Sport,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{HashMap, HashSet};

/// §49 coverage for the store's merged tables. See [`super::coverage_report`] for the contract this
/// fills, including the cohort filter it applies to the athlete-derived columns.
pub(super) fn run(store: &Store, grad_year: Option<i16>) -> ReportResult<Outcome> {
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let event_ids: HashSet<String> = store
        .scan::<CanonicalEvent>(Table::Events)?
        .into_iter()
        .map(|event| event.id.as_str().to_string())
        .collect();
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;

    let school_state = school_state_index(&schools);
    let coach_schools: HashSet<&str> = coaches.iter().map(|coach| coach.school.as_str()).collect();
    let mut sets = SchoolSets::default();
    let mut buckets = seed_buckets();
    let (athletes_read, off_cohort_athletes) = athletes::classify(
        &athletes,
        &school_state,
        &coach_schools,
        grad_year,
        &mut buckets,
        &mut sets.with_athletes,
    );
    let athlete_ids: HashSet<&str> = athletes.iter().map(|athlete| athlete.id.as_str()).collect();
    let (perf_tallies, orphan_performances) =
        athletes::tally_performances(&performances, &athlete_ids, &event_ids);
    let performances_read = athletes::classify_performances(
        &athletes,
        &school_state,
        &perf_tallies,
        &orphan_performances,
        grad_year,
        &mut buckets,
    );
    // Coaches before schools: the school pass counts the sets the coach pass fills.
    classify_coaches(&coaches, &school_state, &mut buckets, &mut sets);
    classify_schools(&schools, &sets, &mut buckets);
    classify_meets(&meets, &mut buckets);
    // Last: `retain_core` deletes exactly the non-core evidence every column above counted.
    count_core(&mut athletes, &school_state, grad_year, &mut buckets);
    let (jurisdictions, gaps) = publish(buckets);

    Ok(Outcome {
        jurisdictions,
        gaps,
        read: CoverageTotals {
            schools: schools.len(),
            athletes: athletes_read,
            coaches: coaches.len(),
            meets: meets.len(),
            performances: performances_read,
        },
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

/// Every published row's bucket, in publication order: [`UsJurisdiction::ALL`], then the unplaced
/// row.
fn jurisdiction_buckets() -> impl Iterator<Item = JurisdictionBucket> {
    UsJurisdiction::ALL
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

/// The core column, counted after every all-sources column is filled.
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

/// The published rows in [`UsJurisdiction::ALL`] order with the unplaceable row last, plus the gap
/// rows each one produced.
fn publish(mut buckets: BucketMap) -> (Vec<JurisdictionCoverage>, Vec<CoverageGap>) {
    let mut jurisdictions = Vec::with_capacity(UsJurisdiction::ALL.len().saturating_add(1));
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
