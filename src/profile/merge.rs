use anyhow::{bail, Result};

use crate::domain::evidence::{EvidenceIssue, ProfileEvidence, ResultEvidence};

pub fn merge_profiles(
    mut left: ProfileEvidence,
    right: ProfileEvidence,
) -> Result<ProfileEvidence> {
    if left.athlete_id != right.athlete_id {
        bail!("cannot merge profiles for different athletes");
    }
    if left.name.value != right.name.value {
        left.issues
            .push(conflict("identity_conflict", "profile names conflict"));
    }
    right
        .teams
        .iter()
        .filter(|value| {
            left.teams.iter().any(|item| {
                item.team_id == value.team_id
                    && (item.name.value != value.name.value
                        || item
                            .location
                            .as_ref()
                            .zip(value.location.as_ref())
                            .is_some_and(|(a, b)| a.value != b.value)
                        || item.level.zip(value.level).is_some_and(|(a, b)| a != b))
            })
        })
        .for_each(|_| {
            left.issues.push(conflict(
                "team_conflict",
                "same team identity has conflicting factual observations",
            ))
        });
    right
        .grades
        .iter()
        .filter(|value| {
            left.grades.iter().any(|item| {
                item.team_id == value.team_id
                    && item.season == value.season
                    && item.grade != value.grade
            })
        })
        .for_each(|_| {
            left.issues.push(conflict(
                "grade_conflict",
                "same team-season has conflicting grades",
            ))
        });
    if !left.graduation_years.is_empty()
        && !right.graduation_years.is_empty()
        && left.graduation_years.iter().all(|item| {
            !right
                .graduation_years
                .iter()
                .any(|other| item.value == other.value)
        })
    {
        left.issues.push(conflict(
            "cohort_conflict",
            "graduation-year witnesses conflict",
        ));
    }
    right
        .sports
        .iter()
        .filter(|value| {
            left.sports
                .iter()
                .any(|item| sport(item) == sport(value) && item != *value)
        })
        .for_each(|_| {
            left.issues.push(conflict(
                "sport_conflict",
                "same sport availability observations conflict",
            ))
        });
    right.teams.into_iter().for_each(|value| {
        if !left.teams.contains(&value) {
            left.teams.push(value);
        }
    });
    right.grades.into_iter().for_each(|value| {
        if !left.grades.contains(&value) {
            left.grades.push(value);
        }
    });
    right.graduation_years.into_iter().for_each(|value| {
        if !left.graduation_years.contains(&value) {
            left.graduation_years.push(value);
        }
    });
    right.sports.into_iter().for_each(|value| {
        if !left.sports.contains(&value) {
            left.sports.push(value);
        }
    });
    right
        .results
        .into_iter()
        .for_each(|value| merge_result(&mut left, value));
    right.issues.into_iter().for_each(|value| {
        if !left.issues.contains(&value) {
            left.issues.push(value);
        }
    });
    right.documents.into_iter().for_each(|value| {
        if !left.documents.contains(&value) {
            left.documents.push(value);
        }
    });
    Ok(left)
}

fn merge_result(profile: &mut ProfileEvidence, result: ResultEvidence) {
    let same_id = profile
        .results
        .iter()
        .filter(|item| item.result_id == result.result_id && item.sport == result.sport)
        .collect::<Vec<_>>();
    if same_id.iter().any(|item| equivalent_result(item, &result)) {
        return;
    }
    if !same_id.is_empty() {
        profile.issues.push(conflict(
            "result_conflict",
            "same result identity has conflicting observations",
        ));
    }
    profile.results.push(result);
}

fn equivalent_result(left: &ResultEvidence, right: &ResultEvidence) -> bool {
    left.result_id == right.result_id
        && left.sport == right.sport
        && left.event_id == right.event_id
        && left.event_name == right.event_name
        && left.event_description == right.event_description
        && left.event_type == right.event_type
        && left.mark == right.mark
        && left.units == right.units
        && left.season == right.season
        && left.team_id == right.team_id
        && left.meet_id == right.meet_id
        && left.meet_name == right.meet_name
        && left.date == right.date
        && left.wind == right.wind
        && left.timing == right.timing
        && left.personal_best == right.personal_best
        && left.season_best == right.season_best
        && left.attribution == right.attribution
        && left.short_code == right.short_code
        && left.result_url == right.result_url
}

fn conflict(code: &str, message: &str) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence: None,
    }
}

fn sport(value: &crate::domain::evidence::SportAvailability) -> crate::domain::evidence::Sport {
    use crate::domain::evidence::SportAvailability;
    match value {
        SportAvailability::ResultsObserved { sport, .. }
        | SportAvailability::EmptyResponse { sport }
        | SportAvailability::Unavailable { sport } => *sport,
    }
}
