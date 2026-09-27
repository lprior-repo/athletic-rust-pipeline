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

struct Scanned {
    schools: Vec<CanonicalSchool>,
    coaches: Vec<CanonicalCoach>,
    meets: Vec<CanonicalMeet>,
    athletes: Vec<CanonicalAthlete>,
    performances: Vec<CanonicalPerformance>,
    event_ids: HashSet<String>,
    unmapped_event_ids: HashSet<String>,
}

impl Scanned {
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

pub(super) fn run(store: &Store, grad_year: Option<i16>) -> ReportResult<Outcome> {
    let mut scanned = Scanned::read(store)?;
    let school_state = school_state_index(&scanned.schools);
    let universe = Published::new(jurisdiction_buckets());
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
    classify_coaches(&scanned.coaches, &school_state, &mut buckets, &mut sets);
    classify_schools(&scanned.schools, &sets, &mut buckets);
    classify_meets(&scanned.meets, &mut buckets);
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

fn publish(mut buckets: BucketMap) -> (Vec<JurisdictionCoverage>, Vec<CoverageGap>) {
    let mut jurisdictions =
        Vec::with_capacity(UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    let mut gap_rows = Vec::new();
    for bucket in jurisdiction_buckets() {
        let (row, counters) = buckets.remove(&bucket).unwrap_or_default().finish(bucket);
        gap_rows.extend(gaps::rows(&row, &counters));
        jurisdictions.push(row);
    }
    (jurisdictions, gap_rows)
}

fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}
