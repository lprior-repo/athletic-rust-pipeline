use super::*;
use census_domain::model::{ReviewCase, SourceObservation};

pub(super) struct ProfileRows<'a> {
    pub bio: &'a Bio,
    pub scope: Scope,
    pub source: &'a SourceRef,
    pub observed_on: &'a str,
}

impl ProfileRows<'_> {
    pub fn locator(&self, path: &str) -> String {
        let source = self
            .source
            .url
            .as_deref()
            .map_or(self.source.id.as_str(), |url| url);
        format!("{source}#{path}")
    }

    pub fn observation(&self, path: &str, school: Option<&str>) -> SourceAthleteObservation {
        SourceAthleteObservation::new(
            SourceNamespace::athletic_net("athlete"),
            self.bio.athlete.id.to_string(),
            self.locator(path),
            self.bio.athlete.name(),
            self.observed_on,
        )
        .with_school(
            school
                .and_then(|id| self.bio.teams.get(id))
                .map(|team| team.school_name.clone()),
        )
        .with_gender(gender_of(&self.bio.athlete.gender).map_or(Gender::Unknown, |gender| gender))
        .with_profile_url(Some(profile_url(self.bio.athlete.id)))
    }

    pub fn reject(&self, accumulated: &mut Accumulator, path: &str, detail: &str) {
        let school = self.bio.athlete.school_id.map(|id| id.to_string());
        self.reject_at_school(accumulated, path, detail, school.as_deref());
    }

    fn reject_at_school(
        &self,
        accumulated: &mut Accumulator,
        path: &str,
        detail: &str,
        school: Option<&str>,
    ) {
        accumulated
            .profile_observations
            .push(SourceObservation::Athlete(self.observation(path, school)));
        self.review(accumulated, path, detail);
    }

    pub fn review(&self, accumulated: &mut Accumulator, path: &str, detail: &str) {
        accumulated.profile_reviews.push(ReviewCase::pending(
            "athleticnet_profile_rejection",
            self.locator(path),
            self.bio.athlete.name(),
            format!(
                "{detail}. Source: {}. Observed on: {}",
                self.locator(path),
                self.observed_on
            ),
        ));
    }

    pub fn retain_grades(&self, accumulated: &mut Accumulator) -> Vec<ObservedGrade> {
        let mut observed = Vec::new();
        let Some(grades) = self.bio.grades.as_ref().filter(|grades| !grades.is_empty()) else {
            self.reject(accumulated, "grades", "No published grade observations");
            return observed;
        };
        for (key, value) in grades {
            let path = format!("grades/{key}");
            match parse_grade(key, *value, self.source) {
                Ok((school, grade)) => {
                    accumulated
                        .unsupported
                        .retain(grade.clone(), self.observation(&path, Some(school)));
                    observed.push(grade);
                }
                Err(reason) => self.reject_at_school(
                    accumulated,
                    &path,
                    &format!("{reason}; published grade {value}"),
                    key.split_once('_').map(|(school, _)| school),
                ),
            }
        }
        observed.sort_by_key(|grade| grade.school_year);
        observed
    }

    pub fn withhold(&self, accumulated: &mut Accumulator, reason: &str) {
        self.reject(accumulated, "athlete", reason);
        match self.scope {
            Scope::TrackField => {
                self.bio
                    .results_tf
                    .iter()
                    .flatten()
                    .enumerate()
                    .for_each(|(position, row)| {
                        self.reject_result(
                            accumulated,
                            position,
                            row.id,
                            &row.result,
                            row.school_id,
                        );
                    })
            }
            Scope::CrossCountry => {
                self.bio
                    .results_xc
                    .iter()
                    .flatten()
                    .enumerate()
                    .for_each(|(position, row)| {
                        self.reject_result(
                            accumulated,
                            position,
                            row.id,
                            &row.result,
                            row.school_id,
                        );
                    })
            }
        }
    }

    pub fn reject_result(
        &self,
        accumulated: &mut Accumulator,
        position: usize,
        id: i64,
        mark: &str,
        school: Option<i64>,
    ) {
        let array = match self.scope {
            Scope::TrackField => "resultsTF",
            Scope::CrossCountry => "resultsXC",
        };
        let path = format!("{array}/{position}");
        let school = school.map(|id| id.to_string());
        self.reject_at_school(
            accumulated,
            &path,
            &format!(
                "Result {id} not canonically admitted; published mark {mark:?}; published SchoolID {school:?}"
            ),
            school.as_deref(),
        );
    }
}

fn parse_grade<'a>(
    key: &'a str,
    value: i64,
    source: &SourceRef,
) -> Result<(&'a str, ObservedGrade), &'static str> {
    let (school, season) = key
        .split_once('_')
        .ok_or("Malformed school/season grade key")?;
    school
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("Invalid grade school identifier")?;
    let grade = u8::try_from(value)
        .ok()
        .and_then(Grade::new)
        .ok_or("Invalid published grade")?;
    let season = season.parse::<i16>().map_err(|_| "Invalid grade season")?;
    let school_year = SchoolYear::containing(season, 5).ok_or("Invalid grade school year")?;
    Ok((
        school,
        ObservedGrade {
            grade,
            school_year,
            source: source.clone(),
        },
    ))
}
