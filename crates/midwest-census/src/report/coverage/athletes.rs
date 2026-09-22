//! The athlete and performance passes: one athlete into one jurisdiction row, and the performance
//! tallies the published athlete ids then join to.

use super::state::{bucket_mut, in_cohort, jurisdiction_of, Bucket, BucketMap, PerfTally};
use super::{JurisdictionCoverage, UNKNOWN_JURISDICTION};
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, Gender, Mark, Sport};
use census_domain::UsJurisdiction;
use std::collections::{BTreeSet, HashMap, HashSet};

/// The athlete pass: cohort filtering, the identity and sport columns, and the two gap counts that
/// need the school and coach tables. Returns `(athletes read, athletes outside the cohort)`.
pub(super) fn classify<'a>(
    athletes: &'a [CanonicalAthlete],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    coach_schools: &HashSet<&str>,
    grad_year: Option<i16>,
    buckets: &mut BucketMap,
    with_athletes: &mut BTreeSet<&'a str>,
) -> (usize, usize) {
    let mut read = 0_usize;
    let mut off_cohort = 0_usize;
    for athlete in athletes {
        if !in_cohort(athlete, grad_year) {
            bump(&mut off_cohort);
            continue;
        }
        bump(&mut read);
        let school = athlete.school.as_str();
        with_athletes.insert(school);
        let bucket = bucket_mut(buckets, jurisdiction_of(school_state, school));
        tally(bucket, athlete);
        if !school_state.contains_key(school) {
            bump(&mut bucket.gaps.missing_school);
        }
        if !coach_schools.contains(school) {
            bump(&mut bucket.gaps.missing_coach);
        }
    }
    (read, off_cohort)
}

/// One athlete into one row.
fn tally(bucket: &mut Bucket, athlete: &CanonicalAthlete) {
    let row = &mut bucket.row;
    bump(&mut row.athletes);
    match athlete.gender {
        Gender::Boys => bump(&mut row.boys),
        Gender::Girls => bump(&mut row.girls),
        Gender::Mixed | Gender::Unknown => bump(&mut row.unknown_gender),
    }
    if athlete.observed_grades.is_empty() {
        bump(&mut row.grad_unresolved);
    } else {
        bump(&mut row.grad_verified);
    }
    if has_conflicting_grade(athlete) {
        bump(&mut row.identity_conflicts);
    }
    for sport in [Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry] {
        if athlete.sports.contains(&sport) {
            bump(sport_column(row, sport));
        }
    }
    tally_profile(row, athlete);
    tally_sources(row, athlete);
}

/// The column one census sport counts in.
fn sport_column(row: &mut JurisdictionCoverage, sport: Sport) -> &mut usize {
    match sport {
        Sport::OutdoorTrack => &mut row.outdoor_track,
        Sport::IndoorTrack => &mut row.indoor_track,
        Sport::CrossCountry => &mut row.cross_country,
    }
}

/// The profile columns: any URL, and the two provider profiles the objective names.
fn tally_profile(row: &mut JurisdictionCoverage, athlete: &CanonicalAthlete) {
    if !athlete.public_profile_urls.is_empty() {
        bump(&mut row.with_profile_url);
    }
    if athlete
        .public_profile_urls
        .iter()
        .any(|url| url.contains("athletic.net"))
    {
        bump(&mut row.with_athletic_net_url);
    }
    if athlete
        .public_profile_urls
        .iter()
        .any(|url| url.contains("milesplit.com"))
    {
        bump(&mut row.with_milesplit_url);
    }
}

/// The provider columns, from evidence and identities rather than from URLs.
fn tally_sources(row: &mut JurisdictionCoverage, athlete: &CanonicalAthlete) {
    if distinct_namespaces(athlete) > 1 {
        bump(&mut row.multisource);
    }
    for source in distinct_evidence_sources(athlete) {
        bump(row.sources.entry(source.to_string()).or_default());
    }
}

/// The adapters that left evidence on one athlete, each counted once however many rows it wrote.
fn distinct_evidence_sources(athlete: &CanonicalAthlete) -> Vec<&str> {
    let mut sources: Vec<&str> = athlete
        .evidence
        .iter()
        .map(|evidence| evidence.source.id.as_str())
        .collect();
    sources.sort_unstable();
    sources.dedup();
    sources
}

/// How many distinct source namespaces one athlete's identities name.
fn distinct_namespaces(athlete: &CanonicalAthlete) -> usize {
    let mut namespaces: Vec<String> = athlete
        .source_identities
        .iter()
        .map(|identity| identity.namespace.to_string())
        .collect();
    namespaces.sort_unstable();
    namespaces.dedup();
    namespaces.len()
}

/// Whether a grade observation implies a graduation year other than the athlete's stored cohort —
/// the disagreement the store's athlete merge records as `Confidence::LOW`.
fn has_conflicting_grade(athlete: &CanonicalAthlete) -> bool {
    athlete
        .observed_grades
        .iter()
        .any(|observation| observation.grad_year() != athlete.grad_year)
}

/// The performance pass: one tally per athlete id, plus the tally for rows whose athlete is not in
/// the athlete table (they cannot be cohort-filtered, so they publish in [`UNKNOWN_JURISDICTION`]).
pub(super) fn tally_performances<'a>(
    performances: &'a [CanonicalPerformance],
    athlete_ids: &HashSet<&str>,
    event_ids: &HashSet<String>,
) -> (HashMap<&'a str, PerfTally>, PerfTally) {
    let mut tallies: HashMap<&str, PerfTally> = HashMap::new();
    let mut orphan = PerfTally::default();
    for performance in performances {
        let tally = if athlete_ids.contains(performance.athlete.as_str()) {
            tallies.entry(performance.athlete.as_str()).or_default()
        } else {
            &mut orphan
        };
        bump(&mut tally.rows);
        if !matches!(performance.mark, Mark::Raw(_)) {
            bump(&mut tally.comparable);
        }
        if !event_ids.contains(performance.event.as_str()) {
            bump(&mut tally.missing_event_context);
        }
    }
    (tallies, orphan)
}

/// Attach the performance tallies to the jurisdiction each published athlete publishes in. Returns
/// the performances the rows then carry.
pub(super) fn classify_performances(
    athletes: &[CanonicalAthlete],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    tallies: &HashMap<&str, PerfTally>,
    orphan: &PerfTally,
    grad_year: Option<i16>,
    buckets: &mut BucketMap,
) -> usize {
    let mut read = 0_usize;
    // The orphan bucket holds rows whose athlete was never stored: they publish as performances and
    // gap counts, never as athletes, because no athlete row exists for those columns to describe.
    {
        let bucket = bucket_mut(buckets, UNKNOWN_JURISDICTION);
        bucket.row.performances = bucket.row.performances.saturating_add(orphan.rows);
        bucket.gaps.missing_event_context = bucket
            .gaps
            .missing_event_context
            .saturating_add(orphan.missing_event_context);
        read = read.saturating_add(orphan.rows);
    }
    for athlete in athletes {
        if !in_cohort(athlete, grad_year) {
            continue;
        }
        let Some(tally) = tallies.get(athlete.id.as_str()) else {
            continue;
        };
        let code = jurisdiction_of(school_state, athlete.school.as_str());
        add_perf(bucket_mut(buckets, code), tally, &mut read);
    }
    read
}

/// One published athlete's tally into that athlete's row, accumulating the performances it carried
/// into `read` — the same rows the published totals then sum back.
fn add_perf(bucket: &mut Bucket, tally: &PerfTally, read: &mut usize) {
    let row = &mut bucket.row;
    if tally.rows > 0 {
        bump(&mut row.with_performance);
    }
    if tally.comparable > 0 {
        bump(&mut row.with_comparable_mark);
    }
    row.performances = row.performances.saturating_add(tally.rows);
    let gaps = &mut bucket.gaps;
    gaps.missing_event_context = gaps
        .missing_event_context
        .saturating_add(tally.missing_event_context);
    *read = read.saturating_add(tally.rows);
}

/// Saturating counter bump. The crate's report helpers are `pub(super)` to `report`, so this part
/// keeps its own copy rather than widening their visibility.
fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}
