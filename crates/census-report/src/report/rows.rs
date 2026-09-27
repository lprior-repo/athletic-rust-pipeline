
use super::notes::bump;
use super::{SportsBreakdown, StateCensus};
use census_domain::model::{CanonicalAthlete, CanonicalCoach, CanonicalSchool, Gender, Sport};
use census_domain::JurisdictionBucket;
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn is_track_or_xc(sport: Sport) -> bool {
    matches!(
        sport,
        Sport::OutdoorTrack | Sport::IndoorTrack | Sport::CrossCountry
    )
}

pub(super) struct RowCounts {
    pub(super) schools: usize,
    pub(super) athletes: usize,
    pub(super) coaches: usize,
    pub(super) coaches_with_email: usize,
    pub(super) dropped: usize,
}

impl RowCounts {
    pub(super) fn of(
        schools: &[CanonicalSchool],
        athletes: &[CanonicalAthlete],
        coaches: &[CanonicalCoach],
        dropped: usize,
    ) -> Self {
        Self {
            schools: schools.len(),
            athletes: athletes.len(),
            coaches: coaches.len(),
            coaches_with_email: coaches
                .iter()
                .filter(|coach| coach.has_published_email())
                .count(),
            dropped,
        }
    }
}

#[derive(Default)]
pub(super) struct Co2027Rollup {
    pub(super) sports: SportsBreakdown,
    pub(super) namespaces: BTreeMap<String, usize>,
    pub(super) grade_evidence_sources: BTreeMap<String, usize>,
    pub(super) multisource: usize,
    pub(super) athletic_net_urls: usize,
}

#[derive(Default)]
pub(super) struct AthleteRollup {
    pub(super) by_state: BTreeMap<JurisdictionBucket, StateCensus>,
    pub(super) by_grad_year: BTreeMap<String, usize>,
    pub(super) co2027: Co2027Rollup,
}

#[derive(Default)]
pub(super) struct CoachRollup {
    pub(super) by_state: BTreeMap<JurisdictionBucket, (usize, usize)>,
    pub(super) roles: BTreeMap<String, usize>,
    pub(super) sports: BTreeMap<String, usize>,
    pub(super) sources: BTreeMap<String, usize>,
}

pub(super) fn school_coach_index(
    coaches: &[CanonicalCoach],
) -> HashMap<&str, (&CanonicalCoach, bool)> {
    let mut index: HashMap<&str, (&CanonicalCoach, bool)> = HashMap::new();
    for coach in coaches {
        if !coach.sport.is_some_and(is_track_or_xc) {
            continue;
        }
        let entry = index.entry(coach.school.as_str()).or_insert((coach, false));
        if coach.has_published_email() {
            entry.1 = true;
        }
    }
    index
}

fn tally_sports(sports: &mut SportsBreakdown, athlete: &CanonicalAthlete) {
    let indoor = athlete.sports.contains(&Sport::IndoorTrack);
    let outdoor = athlete.sports.contains(&Sport::OutdoorTrack);
    let cross_country = athlete.sports.contains(&Sport::CrossCountry);
    match (indoor, outdoor, cross_country) {
        (false, false, false) => bump(&mut sports.none),
        (true, false, false) => bump(&mut sports.indoor_only),
        (false, true, false) => bump(&mut sports.outdoor_only),
        (false, false, true) => bump(&mut sports.cross_country_only),
        _ => bump(&mut sports.multi_sport),
    }
}

pub(super) fn tally_co2027(
    entry: &mut StateCensus,
    co: &mut Co2027Rollup,
    athlete: &CanonicalAthlete,
    school_coach: &HashMap<&str, (&CanonicalCoach, bool)>,
) {
    bump(&mut entry.class_of_2027);
    match athlete.gender {
        Gender::Boys => bump(&mut entry.class_of_2027_boys),
        Gender::Girls => bump(&mut entry.class_of_2027_girls),
        Gender::Mixed | Gender::Unknown => bump(&mut entry.class_of_2027_unknown_gender),
    }
    if !athlete.public_profile_urls.is_empty() {
        bump(&mut entry.class_of_2027_with_profile_url);
    }
    if !athlete.observed_grades.is_empty() {
        bump(&mut entry.class_of_2027_with_grad_year_evidence);
    }
    for observation in &athlete.observed_grades {
        bump(
            co.grade_evidence_sources
                .entry(observation.source.id.clone())
                .or_default(),
        );
    }
    let distinct: BTreeSet<String> = athlete.identities()
        .map(|identity| identity.namespace.to_string())
        .collect();
    if distinct.len() > 1 {
        bump(&mut co.multisource);
        bump(&mut entry.class_of_2027_multisource);
    }
    for namespace in distinct {
        bump(co.namespaces.entry(namespace).or_default());
    }
    if athlete
        .public_profile_urls
        .iter()
        .any(|url| url.contains("athletic.net"))
    {
        bump(&mut co.athletic_net_urls);
    }
    if let Some((_, has_email)) = school_coach.get(athlete.school.as_str()) {
        bump(&mut entry.class_of_2027_with_coach);
        if *has_email {
            bump(&mut entry.class_of_2027_with_coach_email);
        }
    }
    tally_sports(&mut co.sports, athlete);
}

pub(super) fn coach_sport(coach: &CanonicalCoach) -> String {
    coach
        .sport
        .map(|sport| format!("{sport:?}").to_lowercase())
        .unwrap_or_else(|| "school_wide".to_string())
}
