
use super::state::{in_cohort, jurisdiction_of};
use super::CoverageTotals;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{HashMap, HashSet};

pub(super) struct Tables<'a> {
    pub(super) schools: &'a [CanonicalSchool],
    pub(super) athletes: &'a [CanonicalAthlete],
    pub(super) coaches: &'a [CanonicalCoach],
    pub(super) meets: &'a [CanonicalMeet],
    pub(super) performances: &'a [CanonicalPerformance],
}

impl Tables<'_> {
    fn athlete_by_id(&self) -> HashMap<&str, &CanonicalAthlete> {
        self.athletes
            .iter()
            .map(|athlete| (athlete.id.as_str(), athlete))
            .collect()
    }
}

pub(super) struct Published {
    buckets: HashSet<JurisdictionBucket>,
}

impl Published {
    pub(super) fn new(buckets: impl IntoIterator<Item = JurisdictionBucket>) -> Self {
        Self {
            buckets: buckets.into_iter().collect(),
        }
    }

    fn contains(&self, bucket: JurisdictionBucket) -> bool {
        self.buckets.contains(&bucket)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Reads {
    pub(super) published: CoverageTotals,
    pub(super) outside: CoverageTotals,
}

pub(super) fn totals(
    tables: &Tables<'_>,
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    universe: &Published,
    grad_year: Option<i16>,
) -> Reads {
    let athlete_by_id = tables.athlete_by_id();
    let splits = Splits {
        schools: schools(tables.schools, universe),
        athletes: athletes(tables.athletes, school_state, universe, grad_year),
        coaches: coaches(tables.coaches, school_state, universe),
        meets: meets(tables.meets, universe),
        performances: performances(
            tables.performances,
            &athlete_by_id,
            school_state,
            universe,
            grad_year,
        ),
    };
    Reads {
        published: splits.published(),
        outside: splits.outside(),
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Splits {
    schools: ScopeSplit,
    athletes: ScopeSplit,
    coaches: ScopeSplit,
    meets: ScopeSplit,
    performances: ScopeSplit,
}

impl Splits {
    fn published(&self) -> CoverageTotals {
        CoverageTotals {
            schools: self.schools.published,
            athletes: self.athletes.published,
            coaches: self.coaches.published,
            meets: self.meets.published,
            performances: self.performances.published,
        }
    }

    fn outside(&self) -> CoverageTotals {
        CoverageTotals {
            schools: self.schools.outside,
            athletes: self.athletes.outside,
            coaches: self.coaches.outside,
            meets: self.meets.outside,
            performances: self.performances.outside,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ScopeSplit {
    published: usize,
    outside: usize,
}

impl ScopeSplit {
    fn add(&mut self, published: bool) {
        let counter = if published {
            &mut self.published
        } else {
            &mut self.outside
        };
        *counter = counter.saturating_add(1);
    }
}

fn schools(schools: &[CanonicalSchool], universe: &Published) -> ScopeSplit {
    let mut split = ScopeSplit::default();
    for school in schools {
        split.add(universe.contains(JurisdictionBucket::from(school.state)));
    }
    split
}

fn coaches(
    coaches: &[CanonicalCoach],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    universe: &Published,
) -> ScopeSplit {
    let mut split = ScopeSplit::default();
    for coach in coaches {
        split.add(universe.contains(jurisdiction_of(school_state, coach.school.as_str())));
    }
    split
}

fn meets(meets: &[CanonicalMeet], universe: &Published) -> ScopeSplit {
    let mut split = ScopeSplit::default();
    for meet in meets {
        split.add(universe.contains(JurisdictionBucket::from(meet.state)));
    }
    split
}

fn athletes(
    athletes: &[CanonicalAthlete],
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    universe: &Published,
    grad_year: Option<i16>,
) -> ScopeSplit {
    let mut split = ScopeSplit::default();
    for athlete in athletes {
        if !in_cohort(athlete, grad_year) {
            continue;
        }
        split.add(universe.contains(jurisdiction_of(school_state, athlete.school.as_str())));
    }
    split
}

fn performances(
    performances: &[CanonicalPerformance],
    athlete_by_id: &HashMap<&str, &CanonicalAthlete>,
    school_state: &HashMap<&str, Option<UsJurisdiction>>,
    universe: &Published,
    grad_year: Option<i16>,
) -> ScopeSplit {
    let mut split = ScopeSplit::default();
    for performance in performances {
        let bucket = match athlete_by_id.get(performance.athlete.as_str()) {
            Some(athlete) if in_cohort(athlete, grad_year) => {
                jurisdiction_of(school_state, athlete.school.as_str())
            }
            Some(_) => continue,
            None => JurisdictionBucket::Unplaced,
        };
        split.add(universe.contains(bucket));
    }
    split
}
