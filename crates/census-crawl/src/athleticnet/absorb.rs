//! One payload's rows: the walk from a decoded bio to canonical rows, refusing the rest.

mod rows;

use super::map::{profile_url, school_for, Accumulator, Stats};
use super::parse::{gender_of, Bio};
use super::{Scope, Target};
use census_domain::model::{
    AthleteId, CanonicalAthlete, Evidence, Gender, GradYear, Grade, ObservedGrade, SchoolId,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use std::collections::HashMap;

/// Absorb one payload; returns the number of result rows stored.
#[allow(clippy::too_many_arguments)]
pub(super) fn absorb(
    bio: &Bio,
    scope: Scope,
    target: &Target,
    source: &SourceRef,
    observed_on: &str,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, SchoolId>,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> u64 {
    let name = bio.athlete.name();
    let Some(gender) = gender_of(&bio.athlete.gender) else {
        stats.gender_unknown = stats.gender_unknown.saturating_add(1);
        return 0;
    };
    let observed_grades = grade_observations(bio, source);
    let Some(latest) = observed_grades.last().cloned() else {
        stats.athletes_without_grade = stats.athletes_without_grade.saturating_add(1);
        return 0;
    };
    let grad_year = latest.grad_year();

    let Some(athlete_school) = bio.athlete.school_id else {
        stats.athletes_without_school = stats.athletes_without_school.saturating_add(1);
        return 0;
    };
    let mut ctx = Ctx {
        source,
        observed_on,
        index,
        resolved,
        stats,
        accumulated,
        target,
        school_names: school_names(bio),
        observed_grades,
        seasons: season_sports(bio),
    };
    let Some(school) = ctx.canonical_school(&athlete_school.to_string()) else {
        return 0;
    };
    let athlete_id = ctx.athlete_id(&school, &name, grad_year, gender);
    match scope {
        Scope::TrackField => ctx.track_rows(bio, &athlete_id, gender),
        Scope::CrossCountry => ctx.cross_rows(bio, &athlete_id, gender),
    }
}

/// One payload's walk: the identity it resolved, the schools its rows name, and where rows go.
struct Ctx<'a> {
    source: &'a SourceRef,
    observed_on: &'a str,
    index: &'a SchoolIndex,
    resolved: &'a mut HashMap<String, SchoolId>,
    stats: &'a mut Stats,
    accumulated: &'a mut Accumulator,
    target: &'a Target,
    school_names: HashMap<String, &'a str>,
    observed_grades: Vec<ObservedGrade>,
    seasons: HashMap<(i64, i16), Option<Sport>>,
}

impl<'a> Ctx<'a> {
    /// The canonical school a payload names, counting a payload whose state cannot be resolved.
    fn canonical_school(&mut self, school_id: &str) -> Option<SchoolId> {
        let school = school_for(
            school_id,
            self.target.state,
            &self.school_names,
            self.index,
            self.resolved,
            self.source,
            self.observed_on,
            self.stats,
            self.accumulated,
        );
        if school.is_none() {
            self.stats.rows_without_state = self.stats.rows_without_state.saturating_add(1);
        }
        school
    }

    /// The athlete every row of this payload hangs off, minted once per (athlete id, school).
    fn athlete_id(
        &mut self,
        school: &SchoolId,
        name: &str,
        grad_year: GradYear,
        gender: Gender,
    ) -> AthleteId {
        let key = format!("{}:{}", self.target.athlete_id, school.as_str());
        let id = match self.accumulated.athletes.get(&key) {
            Some(existing) => existing.id.clone(),
            None => {
                let mut athlete = CanonicalAthlete::new(school, name, grad_year, gender,
                    SourceIdentity::new(SourceNamespace::athletic_net("athlete"), self.target.athlete_id.to_string())
                        .with_url(profile_url(self.target.athlete_id)));
                athlete
                    .public_profile_urls
                    .push(profile_url(self.target.athlete_id));
                let id = athlete.id.clone();
                self.accumulated.athletes.insert(key.clone(), athlete);
                id
            }
        };
        let observed_grades = self.observed_grades.clone();
        if let Some(athlete) = self.accumulated.athletes.get_mut(&key) {
            athlete.observed_grades = observed_grades;
            athlete.evidence = vec![Evidence::fetched(self.source.clone(), self.observed_on)];
            athlete.source_identities = vec![SourceIdentity {
                namespace: SourceNamespace::AthleticNet {
                    kind: "athlete".to_string(),
                },
                id: self.target.athlete_id.to_string(),
                url: Some(profile_url(self.target.athlete_id)),
            }];
        }
        id
    }
}

/// Grade observations: `grades` maps `"<SchoolID>_<SeasonID>"` to the grade that season, which is
/// what makes a class year derived rather than assumed.
fn grade_observations(bio: &Bio, source: &SourceRef) -> Vec<ObservedGrade> {
    let mut observed: Vec<ObservedGrade> = Vec::new();
    for (key, grade) in bio.grades.iter().flatten() {
        let Some((_, season)) = key.split_once('_') else {
            continue;
        };
        let (Ok(grade), Ok(season)) = (u8::try_from(*grade), season.parse::<i16>()) else {
            continue;
        };
        let Some(grade) = Grade::new(grade) else {
            continue;
        };
        // A season the domain will not place is dropped rather than recorded as the year a grade was
        // observed in: the observation is evidence, and no source published that year.
        let Some(school_year) = SchoolYear::containing(season, 5) else {
            continue;
        };
        observed.push(ObservedGrade {
            grade,
            school_year,
            source: source.clone(),
        });
    }
    observed.sort_by_key(|observation| observation.school_year);
    observed
}

/// School names are published once per payload, keyed by the school id the rows carry.
fn school_names(bio: &Bio) -> HashMap<String, &str> {
    bio.teams
        .iter()
        .map(|(id, team)| (id.clone(), team.school_name.as_str()))
        .collect()
}

/// Season entries are the only place the indoor/outdoor split is published, keyed by the
/// payload's school + season.
fn season_sports(bio: &Bio) -> HashMap<(i64, i16), Option<Sport>> {
    bio.seasons
        .iter()
        .map(|season| ((season.school_id, season.season_id), season.sport()))
        .collect()
}
