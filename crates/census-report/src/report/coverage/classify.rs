use super::super::is_core_evidenced;
use super::gaps;
use super::reads::{self, Published, Tables};
use super::state::{
    bucket_mut, in_requested_year, jurisdiction_of, school_state_index, Bucket, BucketMap, Outcome,
    SchoolSets,
};
use super::{athletes, CoverageGap, JurisdictionCoverage};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CoachRole, EventKind, Sport,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{HashMap, HashSet};

struct Scanned<'a> {
    schools: Vec<CanonicalSchool>,
    coaches: &'a [CanonicalCoach],
    meets: &'a [CanonicalMeet],
    athletes: Vec<CanonicalAthlete>,
    aliases: &'a HashMap<String, String>,
    performances: &'a [CanonicalPerformance],
    event_ids: HashSet<String>,
    unmapped_event_ids: HashSet<String>,
}

impl<'a> Scanned<'a> {
    fn of(dataset: &'a ExportDataset) -> Self {
        let (event_ids, unmapped_event_ids) = event_indices(dataset);
        Self {
            schools: dataset.schools.values().cloned().collect(),
            coaches: &dataset.coaches,
            meets: &dataset.meets,
            athletes: super::super::derivation::collapse_athletes(
                &dataset.athletes,
                &dataset.canonical_aliases,
            ),
            aliases: &dataset.canonical_aliases,
            performances: &dataset.performances,
            event_ids,
            unmapped_event_ids,
        }
    }

    fn tables(&self) -> Tables<'_> {
        Tables {
            schools: &self.schools,
            athletes: &self.athletes,
            aliases: self.aliases,
            coaches: self.coaches,
            meets: self.meets,
            performances: self.performances,
        }
    }
}

fn event_indices(dataset: &ExportDataset) -> (HashSet<String>, HashSet<String>) {
    let event_ids = dataset
        .events
        .iter()
        .map(|event| event.id.as_str().to_string())
        .collect();
    let unmapped = dataset
        .events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unmapped { .. }))
        .map(|event| event.id.as_str().to_string())
        .collect();
    (event_ids, unmapped)
}

pub(super) fn run(dataset: &ExportDataset, grad_year: Option<i16>) -> Outcome {
    let scanned = Scanned::of(dataset);
    let school_state = school_state_index(&scanned.schools);
    let universe = Published::new(jurisdiction_buckets());
    let reads = reads::totals(&scanned.tables(), &school_state, &universe, grad_year);

    let mut sets = SchoolSets::default();
    let mut buckets = seed_buckets();
    let placement = athletes::Placement {
        school_state: &school_state,
        grad_year,
    };
    let off_cohort_athletes = classify_population(&scanned, &placement, &mut buckets, &mut sets);
    classify_coaches(scanned.coaches, &school_state, &mut buckets, &mut sets);
    classify_schools(&scanned.schools, &sets, &mut buckets);
    classify_meets(scanned.meets, &mut buckets);
    count_core(&scanned.athletes, &school_state, grad_year, &mut buckets);
    let (jurisdictions, gaps) = publish(buckets);

    Outcome {
        jurisdictions,
        gaps,
        read: reads.published,
        outside_scope: reads.outside,
        off_cohort_athletes,
    }
}

fn classify_population<'a>(
    scanned: &'a Scanned<'_>,
    placement: &athletes::Placement<'_, '_>,
    buckets: &mut BucketMap,
    sets: &mut SchoolSets<'a>,
) -> usize {
    let coach_schools = scanned
        .coaches
        .iter()
        .map(|coach| coach.school.as_str())
        .collect();
    let off_cohort = athletes::classify(
        &scanned.athletes,
        placement,
        &coach_schools,
        buckets,
        &mut sets.with_athletes,
    );
    classify_performances(scanned, placement, buckets);
    off_cohort
}

fn classify_performances(
    scanned: &Scanned<'_>,
    placement: &athletes::Placement<'_, '_>,
    buckets: &mut BucketMap,
) {
    let (tallies, orphan) = performance_tallies(scanned);
    athletes::classify_performances(&scanned.athletes, placement, &tallies, &orphan, buckets);
}

fn performance_tallies<'a>(
    scanned: &'a Scanned<'_>,
) -> (
    HashMap<&'a str, super::state::PerfTally>,
    super::state::PerfTally,
) {
    let athlete_ids = scanned
        .athletes
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    athletes::tally_performances(
        scanned.performances,
        &athlete_ids,
        &scanned.event_ids,
        &scanned.unmapped_event_ids,
        scanned.aliases,
    )
}

fn seed_buckets() -> BucketMap {
    let mut buckets = BucketMap::new();
    for bucket in jurisdiction_buckets() {
        buckets.insert(bucket, Bucket::default());
    }
    buckets
}

fn jurisdiction_buckets() -> impl Iterator<Item = JurisdictionBucket> {
    UsJurisdiction::CENSUS_SCOPE
        .iter()
        .copied()
        .map(JurisdictionBucket::from)
        .chain(std::iter::once(JurisdictionBucket::Unplaced))
}

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
        if coach.has_published_email() {
            bump(&mut bucket.row.coaches_with_email);
            sets.with_coach_email.insert(school);
        }
        classify_coach_program(coach, sets);
    }
}

fn classify_coach_program<'a>(coach: &'a CanonicalCoach, sets: &mut SchoolSets<'a>) {
    if coach.role != CoachRole::HeadCoach {
        return;
    }
    let school = coach.school.as_str();
    match coach.sport {
        Some(Sport::OutdoorTrack | Sport::IndoorTrack) => {
            sets.with_tf_coach.insert(school);
        }
        Some(Sport::CrossCountry) => {
            sets.with_xc_coach.insert(school);
        }
        Some(Sport::Unknown) | None => {}
    }
}

fn classify_meets(meets: &[CanonicalMeet], buckets: &mut BucketMap) {
    for meet in meets {
        bump(
            &mut bucket_mut(buckets, JurisdictionBucket::from(meet.state))
                .row
                .meets,
        );
    }
}

fn count_core(
    athletes: &[CanonicalAthlete],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    grad_year: Option<i16>,
    buckets: &mut BucketMap,
) {
    for athlete in athletes {
        if !in_requested_year(athlete, grad_year) || !is_core_evidenced(athlete) {
            continue;
        }
        let bucket = jurisdiction_of(school_state, athlete.school.as_str());
        bump(&mut bucket_mut(buckets, bucket).row.athletes_core);
    }
}

fn publish(mut buckets: BucketMap) -> (Vec<JurisdictionCoverage>, Vec<CoverageGap>) {
    let mut jurisdictions =
        Vec::with_capacity(UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    let mut gap_rows = Vec::new();
    for bucket in jurisdiction_buckets() {
        let (row, counters) = buckets
            .remove(&bucket)
            .map_or(Default::default(), core::convert::identity)
            .finish(bucket);
        gap_rows.extend(gaps::rows(&row, &counters));
        jurisdictions.push(row);
    }
    (jurisdictions, gap_rows)
}

fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}
