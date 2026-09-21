//! One payload's rows: the walk from a decoded bio to canonical rows, refusing the rest.

use super::map::{
    grade_in, meet_for, profile_url, school_for, store_performance, Accumulator, PerformanceInput,
    Stats,
};
use super::parse::{gender_of, parse_mark, round_of, timing_of, Bio};
use super::{Scope, Target};
use crate::model::{
    AthleteId, CanonicalAthlete, EventKind, Evidence, Grade, ObservedGrade, SchoolId, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::school_index::SchoolIndex;
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
        stats.gender_unknown += 1;
        return 0;
    };

    // Grade observations: `grades` maps "<SchoolID>_<SeasonID>" to the grade that season, which is
    // what makes a class year derived rather than assumed.
    let mut observed_grades: Vec<ObservedGrade> = Vec::new();
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
        observed_grades.push(ObservedGrade {
            grade,
            school_year: SchoolYear::containing(season, 5),
            source: source.clone(),
        });
    }
    observed_grades.sort_by_key(|observation| observation.school_year);
    let Some(latest) = observed_grades.last().cloned() else {
        stats.athletes_without_grade += 1;
        return 0;
    };
    let grad_year = latest.grad_year();

    let Some(athlete_school) = bio.athlete.school_id else {
        stats.athletes_without_school += 1;
        return 0;
    };
    // School names are published once per payload, keyed by the school id the rows carry.
    let school_names: HashMap<String, &str> = bio
        .teams
        .iter()
        .map(|(id, team)| (id.clone(), team.school_name.as_str()))
        .collect();
    let Some(school) = school_for(
        &athlete_school.to_string(),
        target.state.as_deref(),
        &school_names,
        index,
        resolved,
        source,
        observed_on,
        stats,
        accumulated,
    ) else {
        stats.rows_without_state += 1;
        return 0;
    };

    let athlete_id: AthleteId =
        match accumulated
            .athletes
            .get(&format!("{}:{}", target.athlete_id, school.as_str()))
        {
            Some(existing) => existing.id.clone(),
            None => {
                let mut athlete = CanonicalAthlete::new(&school, &name, grad_year, gender);
                athlete
                    .public_profile_urls
                    .push(profile_url(target.athlete_id));
                let id = athlete.id.clone();
                accumulated.athletes.insert(
                    format!("{}:{}", target.athlete_id, school.as_str()),
                    athlete,
                );
                id
            }
        };
    if let Some(athlete) =
        accumulated
            .athletes
            .get_mut(&format!("{}:{}", target.athlete_id, school.as_str()))
    {
        athlete.observed_grades = observed_grades.clone();
        athlete.evidence = vec![Evidence::fetched(source.clone(), observed_on)];
        athlete.source_identities = vec![SourceIdentity {
            namespace: SourceNamespace::AthleticNet {
                kind: "athlete".to_string(),
            },
            id: target.athlete_id.to_string(),
            url: Some(profile_url(target.athlete_id)),
        }];
    }

    // Season entries are the only place the indoor/outdoor split is published, keyed by the
    // athlete's school + season.
    let seasons: HashMap<(i64, i16), Option<Sport>> = bio
        .seasons
        .iter()
        .map(|season| ((season.school_id, season.season_id), season.sport()))
        .collect();

    let mut rows = 0u64;
    match scope {
        Scope::TrackField => {
            let labels: HashMap<i64, &str> = bio
                .events
                .iter()
                .flatten()
                .map(|event| (event.id, event.label.as_str()))
                .collect();
            for row in bio.results_tf.iter().flatten() {
                stats.rows_seen += 1;
                let Some(season_id) = row.season_id else {
                    stats.rows_no_season += 1;
                    continue;
                };
                let Some(sport) = seasons
                    .get(&(row.school_id.unwrap_or_default(), season_id))
                    .copied()
                    .flatten()
                else {
                    *stats
                        .rows_unknown_season
                        .entry(season_id.to_string())
                        .or_default() += 1;
                    continue;
                };
                let Some(label) = row.event_id.and_then(|id| labels.get(&id).copied()) else {
                    stats.rows_no_event += 1;
                    continue;
                };
                let kind = EventKind::from_source_label(label);
                let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
                    stats.rows_no_mark += 1;
                    continue;
                };
                let Some(meet) = meet_for(
                    row.meet_id,
                    bio,
                    target.state.as_deref(),
                    source,
                    observed_on,
                    accumulated,
                ) else {
                    stats.rows_unknown_meet += 1;
                    continue;
                };
                let Some(date) = row.date().or_else(|| Some(meet.date.clone())) else {
                    stats.rows_unknown_meet += 1;
                    continue;
                };
                let Some(school) = school_for(
                    &row.school_id.unwrap_or_default().to_string(),
                    target.state.as_deref(),
                    &school_names,
                    index,
                    resolved,
                    source,
                    observed_on,
                    stats,
                    accumulated,
                ) else {
                    stats.rows_without_state += 1;
                    continue;
                };
                let school_year = SchoolYear::containing(season_id, 5);
                let grade = grade_in(&observed_grades, school_year);
                store_performance(
                    accumulated,
                    source,
                    observed_on,
                    PerformanceInput {
                        athlete: &athlete_id,
                        school: &school,
                        meet: &meet,
                        kind: &kind,
                        sport,
                        gender,
                        school_year,
                        grade,
                        date,
                        mark,
                        wind_mps: row.wind,
                        place: row.place.as_deref(),
                        round: round_of(row.round.as_deref()),
                        timing: timing_of(row.fat, auto),
                        division: row.division.clone(),
                        source_key: format!("athleticnet:{}-{}", target.athlete_id, row.id),
                        label: Some(label),
                    },
                );
                rows += 1;
                stats.rows_absorbed += 1;
            }
        }
        Scope::CrossCountry => {
            for row in bio.results_xc.iter().flatten() {
                stats.rows_seen += 1;
                let Some(season_id) = row.season_id else {
                    stats.rows_no_season += 1;
                    continue;
                };
                let kind = EventKind::CrossCountry;
                let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
                    stats.rows_no_mark += 1;
                    continue;
                };
                let Some(meet) = meet_for(
                    row.meet_id,
                    bio,
                    target.state.as_deref(),
                    source,
                    observed_on,
                    accumulated,
                ) else {
                    stats.rows_unknown_meet += 1;
                    continue;
                };
                let date = meet.date.clone();
                let Some(school) = school_for(
                    &row.school_id.unwrap_or_default().to_string(),
                    target.state.as_deref(),
                    &school_names,
                    index,
                    resolved,
                    source,
                    observed_on,
                    stats,
                    accumulated,
                ) else {
                    stats.rows_without_state += 1;
                    continue;
                };
                // A cross-country season is a fall season, so its school year starts in the same
                // calendar year the season is named for.
                let school_year = SchoolYear::containing(season_id, 9);
                let grade = grade_in(&observed_grades, school_year);
                store_performance(
                    accumulated,
                    source,
                    observed_on,
                    PerformanceInput {
                        athlete: &athlete_id,
                        school: &school,
                        meet: &meet,
                        kind: &kind,
                        sport: Sport::CrossCountry,
                        gender,
                        school_year,
                        grade,
                        date,
                        mark,
                        wind_mps: None,
                        place: row.place.as_deref(),
                        round: None,
                        timing: timing_of(0, auto),
                        division: row
                            .division
                            .clone()
                            .or_else(|| row.distance.map(|metres| format!("{metres}m"))),
                        source_key: format!("athleticnet:{}-{}", target.athlete_id, row.id),
                        label: None,
                    },
                );
                rows += 1;
                stats.rows_absorbed += 1;
            }
        }
    }
    rows
}
