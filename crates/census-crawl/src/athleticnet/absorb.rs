mod observations;
mod rows;

#[cfg(test)]
mod tests;

use super::map::{profile_url, school_for, Accumulator, Stats};
use super::parse::{gender_of, Bio};
use super::{Scope, Target};
use census_domain::model::{
    AthleteId, CanonicalAthlete, Evidence, Gender, GradYear, Grade, ObservedGrade, SchoolId,
    SchoolYear, SourceAthleteObservation, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use observations::ProfileRows;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum AbsorbOutcome {
    Complete { rows: u64 },
    Partial { rows: u64 },
    Withheld { reason: &'static str },
}

impl AbsorbOutcome {
    pub fn rows(&self) -> u64 {
        match self {
            Self::Complete { rows } | Self::Partial { rows } => *rows,
            Self::Withheld { .. } => 0,
        }
    }
}
pub(super) struct AbsorbContext<'a> {
    pub source: &'a SourceRef,
    pub observed_on: &'a str,
    pub performance_as_of: chrono::NaiveDate,
    pub index: &'a SchoolIndex,
    pub resolved: &'a mut HashMap<String, SchoolId>,
    pub stats: &'a mut Stats,
    pub accumulated: &'a mut Accumulator,
}

pub(super) fn absorb(
    bio: &Bio,
    scope: Scope,
    target: &Target,
    ctx: AbsorbContext<'_>,
) -> crate::CrawlResult<AbsorbOutcome> {
    let published = ProfileRows {
        bio,
        scope,
        source: ctx.source,
        observed_on: ctx.observed_on,
    };
    if bio.athlete.id != target.athlete_id {
        let detail = format!(
            "Requested athlete {} but returned athlete {}",
            target.athlete_id, bio.athlete.id
        );
        published.review(ctx.accumulated, "athlete/IDAthlete", &detail);
        return Err(crate::CrawlError::Schema {
            url: published.locator("athlete/IDAthlete"),
            detail,
        });
    }
    let reviews_before = ctx.accumulated.profile_reviews.len();
    let unsupported_before = ctx.accumulated.unsupported.len();
    let observed_grades = published.retain_grades(ctx.accumulated);
    let (gender, grad_year, school) = match admission(bio, &observed_grades, ctx.stats) {
        Ok(admitted) => admitted,
        Err(reason) => {
            published.withhold(ctx.accumulated, reason);
            return Ok(AbsorbOutcome::Withheld { reason });
        }
    };
    let mut ctx = Ctx {
        source: ctx.source,
        observed_on: ctx.observed_on,
        performance_as_of: ctx.performance_as_of,
        index: ctx.index,
        resolved: ctx.resolved,
        stats: ctx.stats,
        accumulated: ctx.accumulated,
        target,
        source_athlete: SourceIdentity::new(
            SourceNamespace::athletic_net("athlete"),
            bio.athlete.id.to_string(),
        )
        .with_url(profile_url(bio.athlete.id)),
        school_names: school_names(bio),
        observed_grades,
        seasons: season_sports(bio),
    };
    absorb_admitted(
        &published,
        &mut ctx,
        &school,
        (grad_year, gender),
        (reviews_before, unsupported_before),
    )
}

fn absorb_admitted(
    published: &ProfileRows<'_>,
    ctx: &mut Ctx<'_>,
    school: &str,
    admitted: (GradYear, Gender),
    before: (usize, usize),
) -> crate::CrawlResult<AbsorbOutcome> {
    let observed_school = published
        .bio
        .teams
        .get(school)
        .map(|team| team.school_name.clone());
    let Some(school) = ctx.canonical_school(school) else {
        if ctx.target.state.is_none() {
            let count = match published.scope {
                Scope::TrackField => published.bio.results_tf.as_ref().map_or(0, Vec::len),
                Scope::CrossCountry => published.bio.results_xc.as_ref().map_or(0, Vec::len),
            };
            let count = u64::try_from(count).map_or(u64::MAX, |value| value);
            ctx.stats.rows_without_state = ctx.stats.rows_without_state.saturating_add(count);
            ctx.stats.rows_seen = ctx.stats.rows_seen.saturating_add(count);
        }
        let reason = "Unresolved profile school or state";
        published.withhold(ctx.accumulated, reason);
        return Ok(AbsorbOutcome::Withheld { reason });
    };
    let (grad_year, gender) = admitted;
    let athlete_id = ctx.athlete_id(
        &school,
        &published.bio.athlete.name(),
        grad_year,
        gender,
        observed_school,
    )?;
    let rows = match published.scope {
        Scope::TrackField => ctx.track_rows(published.bio, &athlete_id, gender),
        Scope::CrossCountry => ctx.cross_rows(published.bio, &athlete_id, gender),
    };
    let missing = match published.scope {
        Scope::TrackField => published.bio.results_tf.is_none(),
        Scope::CrossCountry => published.bio.results_xc.is_none(),
    };
    if missing {
        published.withhold(
            ctx.accumulated,
            "Published results field is absent, not an empty result set",
        );
    }
    Ok(
        if ctx.accumulated.profile_reviews.len() > before.0
            || ctx.accumulated.unsupported.len() > before.1
        {
            AbsorbOutcome::Partial { rows }
        } else {
            AbsorbOutcome::Complete { rows }
        },
    )
}

fn admission(
    bio: &Bio,
    observed: &[ObservedGrade],
    stats: &mut Stats,
) -> Result<(Gender, GradYear, String), &'static str> {
    let Some(gender) = gender_of(&bio.athlete.gender) else {
        stats.gender_unknown = stats.gender_unknown.saturating_add(1);
        return Err("Unknown published gender");
    };
    let Some(latest) = observed.last() else {
        stats.athletes_without_grade = stats.athletes_without_grade.saturating_add(1);
        return Err("No valid published grade");
    };
    let grad_year = latest
        .grad_year()
        .ok_or("Unsupported graduation inference")?;
    let Some(school) = bio.athlete.school_id.filter(|school| *school > 0) else {
        stats.athletes_without_school = stats.athletes_without_school.saturating_add(1);
        return Err("Missing or invalid published school");
    };
    Ok((gender, grad_year, school.to_string()))
}

struct Ctx<'a> {
    source: &'a SourceRef,
    observed_on: &'a str,
    performance_as_of: chrono::NaiveDate,
    index: &'a SchoolIndex,
    resolved: &'a mut HashMap<String, SchoolId>,
    stats: &'a mut Stats,
    accumulated: &'a mut Accumulator,
    target: &'a Target,
    source_athlete: SourceIdentity,
    school_names: HashMap<String, &'a str>,
    observed_grades: Vec<ObservedGrade>,
    seasons: HashMap<(i64, i16), Option<Sport>>,
}

impl<'a> Ctx<'a> {
    fn canonical_school(&mut self, school_id: &str) -> Option<SchoolId> {
        school_for(
            school_id,
            self.target.state,
            &self.school_names,
            self.index,
            self.resolved,
            self.source,
            self.observed_on,
            self.stats,
            self.accumulated,
        )
    }

    fn athlete_id(
        &mut self,
        school: &SchoolId,
        name: &str,
        grad_year: GradYear,
        gender: Gender,
        observed_school: Option<String>,
    ) -> crate::CrawlResult<AthleteId> {
        let key = format!("{}:{}", self.target.athlete_id, school.as_str());
        let id = match self.accumulated.athletes.get(&key) {
            Some(existing) => existing.id.clone(),
            None => {
                let mut athlete = CanonicalAthlete::new(
                    school,
                    name,
                    grad_year,
                    gender,
                    self.source_athlete.clone(),
                );
                athlete
                    .public_profile_urls
                    .push(profile_url(self.target.athlete_id));
                let id = athlete.id.clone();
                self.accumulated.athletes.insert(key.clone(), athlete);
                id
            }
        };
        if let Some(athlete) = self.accumulated.athletes.get_mut(&key) {
            for observation in &self.observed_grades {
                if !athlete.observed_grades.contains(observation) {
                    athlete.observed_grades.push(observation.clone());
                }
            }
            let evidence = Evidence::parsed(self.source.clone(), self.observed_on);
            if !athlete.evidence.contains(&evidence) {
                athlete.evidence.push(evidence);
            }
            let observation = SourceAthleteObservation::of_athlete(
                &self.source_athlete.namespace,
                athlete,
                observed_school,
                self.observed_on,
            )
            .ok_or_else(|| crate::CrawlError::Invariant {
                detail: "admitted NET profile lacks its provider identity".into(),
            })?;
            self.accumulated
                .profile_observations
                .try_reserve(1)
                .map_err(|_| crate::CrawlError::Resource {
                    resource: "NET profile observations",
                    requested: usize::MAX,
                    limit: crate::net::MAX_BODY_BYTES,
                })?;
            self.accumulated.profile_observations.push(
                census_domain::model::SourceObservation::Athlete(observation),
            );
        }
        Ok(id)
    }
}

fn school_names(bio: &Bio) -> HashMap<String, &str> {
    bio.teams
        .iter()
        .map(|(id, team)| (id.clone(), team.school_name.as_str()))
        .collect()
}

fn season_sports(bio: &Bio) -> HashMap<(i64, i16), Option<Sport>> {
    bio.seasons
        .iter()
        .map(|season| ((season.school_id, season.season_id), season.sport()))
        .collect()
}
