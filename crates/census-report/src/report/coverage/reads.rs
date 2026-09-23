//! The read side of the §49 reconciliation: what the merged tables hold for the rows the report
//! publishes, counted from the tables themselves rather than summed from the rows.
//!
//! Two filters define the side both halves of [`CoverageReport::reconcile`] count under:
//!
//! * the **run scope** ([`Published`]) — the census run scope plus the unplaced row. A row whose
//!   jurisdiction a run never covers (Alaska and Hawaii, where a national all-sources wave can still
//!   leave data behind) publishes no row, so it takes no denominator either; counting it here is what
//!   unbalanced the reconciliation.
//! * the **cohort** (`grad_year`) — the graduation year the athlete and performance columns measure.
//!
//! The counting lives here rather than inside the passes that fill the rows, so a row a pass loses or
//! invents still leaves the two sides apart instead of moving both at once.
//!
//! [`CoverageReport::reconcile`]: super::CoverageReport::reconcile

use super::state::{in_cohort, jurisdiction_of};
use super::CoverageTotals;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{HashMap, HashSet};

/// The merged tables one coverage pass reads: the read side and the passes work from one scan.
pub(super) struct Tables<'a> {
    pub(super) schools: &'a [CanonicalSchool],
    pub(super) athletes: &'a [CanonicalAthlete],
    pub(super) coaches: &'a [CanonicalCoach],
    pub(super) meets: &'a [CanonicalMeet],
    pub(super) performances: &'a [CanonicalPerformance],
}

impl Tables<'_> {
    /// Athlete id to the row of that athlete: the performance side attributes each row through it.
    fn athlete_by_id(&self) -> HashMap<&str, &CanonicalAthlete> {
        self.athletes
            .iter()
            .map(|athlete| (athlete.id.as_str(), athlete))
            .collect()
    }
}

/// The buckets the report publishes a row for.
///
/// Built from the publication order the rows themselves are emitted in, so the read side and the
/// published rows cannot disagree about which jurisdictions this report covers.
pub(super) struct Published {
    buckets: HashSet<JurisdictionBucket>,
}

impl Published {
    /// The published universe over `buckets`, the report's publication order.
    pub(super) fn new(buckets: impl IntoIterator<Item = JurisdictionBucket>) -> Self {
        Self {
            buckets: buckets.into_iter().collect(),
        }
    }

    /// Whether the report publishes a row for `bucket`.
    fn contains(&self, bucket: JurisdictionBucket) -> bool {
        self.buckets.contains(&bucket)
    }
}

/// The read side of the reconciliation, and the sibling count the run scope leaves outside it.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Reads {
    /// What the store held for the rows the report publishes: [`super::CoverageReport::read`].
    pub(super) published: CoverageTotals,
    /// The same five counters for the rows the run scope excludes, which publish in no row.
    pub(super) outside: CoverageTotals,
}

/// Count [`Reads`] from the merged tables, one pass per table.
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

/// One table's scope split per table, named so no caller can pair a table with another's count.
#[derive(Debug, Default, Clone, Copy)]
struct Splits {
    schools: ScopeSplit,
    athletes: ScopeSplit,
    coaches: ScopeSplit,
    meets: ScopeSplit,
    performances: ScopeSplit,
}

impl Splits {
    /// The read side: what the store held for the rows the report publishes.
    fn published(&self) -> CoverageTotals {
        CoverageTotals {
            schools: self.schools.published,
            athletes: self.athletes.published,
            coaches: self.coaches.published,
            meets: self.meets.published,
            performances: self.performances.published,
        }
    }

    /// The rows the run scope excludes: every one of them publishes in no row.
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

/// One scanned table's two sides of the scope line.
#[derive(Debug, Default, Clone, Copy)]
struct ScopeSplit {
    /// Rows whose bucket the report publishes: the read side of the reconciliation.
    published: usize,
    /// Rows whose bucket lies outside the census run scope, which no row publishes.
    outside: usize,
}

impl ScopeSplit {
    /// Count one scanned row on the side the report's universe puts it.
    fn add(&mut self, published: bool) {
        let counter = if published {
            &mut self.published
        } else {
            &mut self.outside
        };
        *counter = counter.saturating_add(1);
    }
}

/// The school table: a school publishes in its own state, or in the unplaced row when it names none.
fn schools(schools: &[CanonicalSchool], universe: &Published) -> ScopeSplit {
    let mut split = ScopeSplit::default();
    for school in schools {
        split.add(universe.contains(JurisdictionBucket::from(school.state)));
    }
    split
}

/// The coach table, placed by the coach's school row: a coach whose school was never stored publishes
/// in the unplaced row, and one whose school sits outside the run scope publishes in no row.
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

/// The meet table: a meet publishes in its venue's jurisdiction, or unplaced when it names none.
fn meets(meets: &[CanonicalMeet], universe: &Published) -> ScopeSplit {
    let mut split = ScopeSplit::default();
    for meet in meets {
        split.add(universe.contains(JurisdictionBucket::from(meet.state)));
    }
    split
}

/// The athlete table under the cohort filter: an athlete outside the cohort is counted by the athlete
/// pass as [`super::CoverageReport::off_cohort_athletes`], and an athlete inside it counts on the side
/// the report's universe puts their school on.
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

/// The performance table, attributed the way the rows attribute it: a row whose athlete was never
/// stored publishes in the unplaced row with the orphan tally, a row of a cohort athlete publishes
/// with that athlete's school, and a row of an athlete outside the cohort publishes nowhere.
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
