//! Roster annotations for the accepted profile: profile URL, competing school, season-and-school
//! history, season-bound grades, and the newest-grade roster verdict.
//!
//! Every item below was previously declared in `export/projection.rs`.

use crate::domain::evidence::ProfileEvidence;
use crate::domain::identity::AthleteId;

pub(super) fn annotations_for(
    profiles: &[ProfileEvidence],
    athlete_id: Option<AthleteId>,
) -> AcceptedAnnotations {
    profiles
        .iter()
        .find(|profile| Some(profile.athlete_id) == athlete_id)
        .map_or_else(AcceptedAnnotations::default, |profile| {
            AcceptedAnnotations {
                profile_url: profile.profile_url.as_str().to_owned(),
                competing_school: competing_school(profile),
                school_history: school_history(profile),
                junior_evidence: junior_evidence(profile),
                junior_status: junior_status(profile),
            }
        })
}

/// Expanded-roster annotations the accepted profile already evidences: its profile URL,
/// the school the athlete competed for in the newest evidenced season, the season-and-school
/// pairs behind that history, every season-bound grade observation, and the roster verdict
/// the newest grade row supports. All empty when no profile was accepted; the school and
/// grade strings are not claims of a complete history — that stays in the retained profile
/// evidence and the JSONL sidecar, including the season/result behind each observation.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct AcceptedAnnotations {
    pub(super) profile_url: String,
    pub(super) competing_school: String,
    pub(super) school_history: String,
    pub(super) junior_evidence: String,
    pub(super) junior_status: String,
}

fn competing_school(profile: &ProfileEvidence) -> String {
    let Some(newest) = profile
        .teams
        .iter()
        .filter_map(|team| team.seasons.iter().copied().max())
        .max()
    else {
        return String::new();
    };
    let mut names: Vec<&str> = profile
        .teams
        .iter()
        .filter(|team| team.seasons.contains(&newest))
        .map(|team| team.name.value.as_str())
        .collect();
    names.sort_unstable();
    names.dedup();
    names.join("; ")
}

/// Every (season, school) pair the profile evidences, newest season first, so a transfer
/// keeps both schools attached to the seasons they were observed in instead of rewriting
/// the school behind earlier performances.
fn school_history(profile: &ProfileEvidence) -> String {
    let mut observations: Vec<(u16, &str)> = profile
        .teams
        .iter()
        .flat_map(|team| {
            team.seasons
                .iter()
                .map(move |season| (*season, team.name.value.as_str()))
        })
        .collect();
    observations.sort_unstable_by(|left, right| right.cmp(left));
    observations.dedup();
    observations
        .into_iter()
        .map(|(season, school)| format!("{season} {school}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn junior_evidence(profile: &ProfileEvidence) -> String {
    let mut observations: Vec<(u16, u8)> = profile
        .grades
        .iter()
        .map(|grade| (grade.season, grade.grade))
        .collect();
    observations.sort_unstable_by(|left, right| right.cmp(left));
    observations.dedup();
    observations
        .into_iter()
        .map(|(season, grade)| format!("grade {grade} @ {season}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// The roster verdict, decided only by the newest season-bound grade row: the season is
/// part of the verdict, and an athlete whose grades never include a standard US high-school
/// grade stays undetermined rather than assumed. Name, age, and graduation year are never
/// consulted.
fn junior_status(profile: &ProfileEvidence) -> String {
    let Some((season, grade)) = profile
        .grades
        .iter()
        .map(|grade| (grade.season, grade.grade))
        .max()
    else {
        return String::new();
    };
    let name = match grade {
        9 => "freshman",
        10 => "sophomore",
        11 => "junior",
        12 => "senior",
        _ => return String::new(),
    };
    format!("{name} @ {season}")
}
