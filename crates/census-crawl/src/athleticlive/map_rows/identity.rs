//! School resolution and identity minting for one mapped row.
//!
//! Part of [`super`]'s row mapping: `resolve_school` decides which consolidated school a published
//! label names, never minting one, and `map_identity` turns a row's facts into the athlete and team
//! the rest of the census keys on. `team_for` mints one team per `(school, sport, gender, school
//! year)`, the same key the roster, association and result-file adapters use.

use std::collections::BTreeMap;

use super::super::map::{ResultStats, RowContext, Writer};
use super::{Mapped, RowIdentity};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalTeam, Gender, GradYear, ObservedGrade, SchoolId,
    SourceIdentity, SourceNamespace,
};

/// Resolve a row's published school label inside the meet's jurisdiction, memoising hits and misses
/// alike: a label is resolved once per run, not once per row.
///
/// No school is ever minted here. A label that names no consolidated school (a club team,
/// `Unattached`) is counted by label and skipped, because the consolidated index is where a school's
/// existence is decided.
pub(super) fn resolve_school(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    label: &str,
) -> Option<SchoolId> {
    let index = writer.index;
    let jurisdiction = context.jurisdiction;
    let resolved = writer
        .resolved
        .entry(label.to_string())
        .or_insert_with(|| index.resolve(jurisdiction, label).map(|(id, _matched)| id));
    if resolved.is_none() {
        let slot = writer
            .stats
            .unresolved
            .entry(label.to_string())
            .or_default();
        *slot = slot.saturating_add(1);
        writer.stats.rows_skipped_unresolved_school = writer
            .stats
            .rows_skipped_unresolved_school
            .saturating_add(1);
    }
    resolved.clone()
}

/// Mint or reuse one row's athlete and team, recording every identity channel it published.
pub(super) fn map_identity(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    school_id: &SchoolId,
    identity: &RowIdentity<'_>,
    row_index: usize,
) -> Mapped {
    let gender = match identity.gender {
        Gender::Unknown => context.gender,
        published => published,
    };
    let team = {
        let entry = team_for(&mut writer.accumulator.teams, school_id, context, gender);
        note_team_ids(entry, context, identity, writer.stats);
        entry.id.clone()
    };
    let athlete = record_athlete(writer, context, school_id, identity, gender, row_index);
    Mapped { athlete, team }
}

/// Record one row's athlete: the graded identity, the published grade as evidence, and the
/// Athletic.net id channels the row carries.
fn record_athlete(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    school_id: &SchoolId,
    identity: &RowIdentity<'_>,
    gender: Gender,
    row_index: usize,
) -> AthleteId {
    let grad_year = GradYear::of(identity.grade, context.school_year);
    let source = identity.an_athlete_id.map_or_else(
        || {
            SourceIdentity::new(
                SourceNamespace::TimerAthlete {
                    provider: context.provider.to_string(),
                },
                format!("{}:row:{row_index}", context.event_key),
            )
        },
        |id| {
            SourceIdentity::new(
                SourceNamespace::LegacyAthleticNet {
                    kind: "athlete".to_string(),
                },
                id.to_string(),
            )
        },
    );
    let athlete_id = CanonicalAthlete::mint(school_id, identity.name, grad_year, gender, &source);
    let entry = writer
        .accumulator
        .athletes
        .entry(athlete_id.as_str().to_string())
        .or_insert_with(|| {
            let mut athlete =
                CanonicalAthlete::new(school_id, identity.name, grad_year, gender, source);
            athlete.sports.push(context.sport);
            athlete.evidence.push(context.evidence.clone());
            athlete
        });
    if !entry.sports.contains(&context.sport) {
        entry.sports.push(context.sport);
    }
    let observation = ObservedGrade {
        grade: identity.grade,
        school_year: context.school_year,
        source: context.source.clone(),
    };
    if !entry.observed_grades.contains(&observation) {
        entry.observed_grades.push(observation);
    }
    if let Some(an_athlete_id) = identity.an_athlete_id {
        writer.stats.rows_with_athlete_id = writer.stats.rows_with_athlete_id.saturating_add(1);
        let row = SourceIdentity::new(
            SourceNamespace::LegacyAthleticNet {
                kind: "athlete".to_string(),
            },
            an_athlete_id.to_string(),
        );
        if !entry.source_identities.contains(&row) {
            entry.source_identities.push(row);
        }
        let profile_url =
            format!("https://www.athletic.net/athlete/{an_athlete_id}/track-and-field");
        if !entry.public_profile_urls.contains(&profile_url) {
            entry.public_profile_urls.push(profile_url);
        }
    }
    athlete_id
}

/// Record the timer and Athletic.net team ids one row publishes on its team.
fn note_team_ids(
    team: &mut CanonicalTeam,
    context: &RowContext<'_>,
    identity: &RowIdentity<'_>,
    stats: &mut ResultStats,
) {
    if let Some(timer_team_id) = identity.timer_team_id {
        stats.rows_with_timer_team_id = stats.rows_with_timer_team_id.saturating_add(1);
        let row = SourceIdentity::new(
            SourceNamespace::TimerTeam {
                provider: context.provider.to_string(),
            },
            timer_team_id.to_string(),
        );
        if !team.source_identities.contains(&row) {
            team.source_identities.push(row);
        }
    }
    if let Some(an_team_id) = identity.an_team_id {
        stats.rows_with_an_team_id = stats.rows_with_an_team_id.saturating_add(1);
        let row = SourceIdentity::new(
            SourceNamespace::LegacyAthleticNet {
                kind: "team".to_string(),
            },
            an_team_id.to_string(),
        );
        if !team.source_identities.contains(&row) {
            team.source_identities.push(row);
        }
    }
}
/// The team a row belongs to: one per (school, sport, gender, school year), minted on the same key
/// the roster, association and result-file adapters use, so one team lands on one canonical team.
fn team_for<'a>(
    teams: &'a mut BTreeMap<String, CanonicalTeam>,
    school: &SchoolId,
    context: &RowContext<'_>,
    gender: Gender,
) -> &'a mut CanonicalTeam {
    let key = format!(
        "{}:{:?}:{gender:?}:{}",
        school.as_str(),
        context.sport,
        context.school_year.get()
    );
    teams.entry(key).or_insert_with(|| CanonicalTeam {
        id: CanonicalTeam::mint(school, context.sport, gender, context.school_year),
        school: school.clone(),
        sport: context.sport,
        gender,
        school_year: context.school_year,
        level: Some("high_school".to_string()),
        source_identities: Vec::new(),
        evidence: vec![context.evidence.clone()],
        retained_conflicts: Vec::new(),
    })
}
