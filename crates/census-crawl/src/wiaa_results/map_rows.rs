//! One mapped row: the school label it resolves to, the athlete entries it observes and the
//! performance entries it mints.

use super::super::Stats;
use super::{MeetContext, RowWriter};
use crate::result_file::ParsedRow;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalPerformance, CanonicalTeam, Evidence, Gender, GradYear,
    Grade, ObservedGrade, SchoolId, SchoolYear, SourceRef, Sport, TeamId,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::HashMap;

/// Count one result row and write its members, returning the athlete rows it produced.
pub(super) fn record_row(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    row_index: usize,
) -> usize {
    writer.stats.rows = writer.stats.rows.saturating_add(1);
    if row.grade.is_some() {
        writer.stats.rows_with_grade = writer.stats.rows_with_grade.saturating_add(1);
    }
    if row.school.trim().is_empty() {
        writer.stats.rows_without_school = writer.stats.rows_without_school.saturating_add(1);
        return 0;
    }
    let Some(school_id) = resolve_school(row, writer.index, writer.resolved, writer.stats) else {
        return 0;
    };

    // Individual rows name an athlete; relay rows name a school and list their legs.
    let members: Vec<(Option<u8>, String, Option<Grade>)> = if row.legs.is_empty() {
        vec![(None, row.name.clone(), row.grade)]
    } else {
        row.legs
            .iter()
            .map(|leg| (Some(leg.position), leg.name.clone(), leg.grade))
            .collect()
    };
    if !row.legs.is_empty() {
        writer.stats.relay_legs = writer.stats.relay_legs.saturating_add(row.legs.len());
    }
    let team_id = team_for(
        &mut writer.accumulator.teams,
        &school_id,
        context.sport,
        context.event.gender,
        context.school_year,
        context.evidence,
    );
    record_members(
        writer, context, row, row_index, &school_id, &team_id, members,
    )
}

/// Resolve a row's published school label, memoising both hits and misses.
fn resolve_school(
    row: &ParsedRow,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, Option<SchoolId>>,
    stats: &mut Stats,
) -> Option<SchoolId> {
    resolved
        .entry(row.school.clone())
        .or_insert_with(
            || match index.resolve(UsJurisdiction::Wisconsin, &row.school) {
                Some((id, kind)) => {
                    let resolved_kind = stats.school_resolved.entry(kind.as_str()).or_default();
                    *resolved_kind = resolved_kind.saturating_add(1);
                    Some(id)
                }
                None => {
                    let unresolved = stats.unresolved.entry(row.school.clone()).or_default();
                    *unresolved = unresolved.saturating_add(1);
                    None
                }
            },
        )
        .clone()
}

/// Write every member of a row: the athlete observation plus the performance entry.
fn record_members(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    row_index: usize,
    school_id: &SchoolId,
    team_id: &TeamId,
    members: Vec<(Option<u8>, String, Option<Grade>)>,
) -> usize {
    let mut athlete_rows = 0usize;
    for (leg_position, member_name, member_grade) in members {
        let Some(grade) = member_grade else {
            continue;
        };
        if member_name.trim().is_empty() {
            continue;
        }
        athlete_rows = athlete_rows.saturating_add(1);
        let athlete_id = record_athlete(writer, context, school_id, &member_name, grade);
        let source_key = performance_key(context, row_index, leg_position);
        let performance_id = CanonicalPerformance::mint(
            &athlete_id,
            &context.meet.id,
            &context.event.kind,
            &context.meet.date,
            &source_key,
        );
        let evidence = performance_evidence(context, row, leg_position);
        writer
            .accumulator
            .performances
            .entry(performance_id.as_str().to_string())
            .or_insert_with(|| CanonicalPerformance {
                id: performance_id,
                athlete: athlete_id,
                team: team_id.clone(),
                event: context.event_id.clone(),
                meet: context.meet.id.clone(),
                date: context.meet.date.clone(),
                mark: row.mark.clone(),
                wind_mps: row.wind_mps,
                place: row.place,
                heat: row.heat.clone(),
                round: context.event.round.clone(),
                timing: Some(context.timing),
                observed_grade: Some(grade),
                evidence: vec![evidence],
                source_key,
                source_athlete: None,
                // A row this mapping just built has been merged with nothing, so it has retained no
                // canonical-id collision: collisions are raised in the store's merge, not here.
                retained_conflicts: Vec::new(),
            });
    }
    athlete_rows
}

/// Record one member's athlete (sports, observed grade, evidence) and return their id.
fn record_athlete(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    school_id: &SchoolId,
    member_name: &str,
    grade: Grade,
) -> AthleteId {
    let grad_year = GradYear::of(grade, context.school_year);
    let athlete_id =
        CanonicalAthlete::mint(school_id, member_name, grad_year, context.event.gender);
    let entry = writer
        .accumulator
        .athletes
        .entry(athlete_id.as_str().to_string())
        .or_insert_with(|| {
            let mut athlete =
                CanonicalAthlete::new(school_id, member_name, grad_year, context.event.gender);
            athlete.sports.push(context.sport);
            athlete
        });
    if !entry.sports.contains(&context.sport) {
        entry.sports.push(context.sport);
    }
    let observation = ObservedGrade {
        grade,
        school_year: context.school_year,
        source: SourceRef::new("wiaa_results", Some(context.artifact.url.clone())),
    };
    if !entry.observed_grades.contains(&observation) {
        entry.observed_grades.push(observation);
    }
    if !entry
        .evidence
        .iter()
        .any(|existing| existing == context.evidence)
    {
        entry.evidence.push(context.evidence.clone());
    }
    athlete_id
}

/// The deterministic per-row performance key: artifact, event, round and row (or relay leg).
fn performance_key(
    context: &MeetContext<'_>,
    row_index: usize,
    leg_position: Option<u8>,
) -> String {
    match leg_position {
        Some(position) => format!(
            "{}:{}:{}:{}:leg{}",
            context.artifact.stem,
            context.event.label,
            context.event.round.as_deref().unwrap_or("final"),
            row_index,
            position
        ),
        None => format!(
            "{}:{}:{}:{}",
            context.artifact.stem,
            context.event.label,
            context.event.round.as_deref().unwrap_or("final"),
            row_index
        ),
    }
}

/// The performance evidence: the meet's, annotated when the row is a relay leg.
fn performance_evidence(
    context: &MeetContext<'_>,
    row: &ParsedRow,
    leg_position: Option<u8>,
) -> Evidence {
    let mut evidence = context.evidence.clone();
    if let Some(position) = leg_position {
        evidence.note = Some(format!(
            "relay leg {position} for {}{}; the published mark is the team's",
            row.school,
            row.heat
                .as_deref()
                .map(|heat| format!(" squad {heat}"))
                .unwrap_or_default()
        ));
    }
    evidence
}

fn team_for(
    teams: &mut HashMap<String, CanonicalTeam>,
    school: &SchoolId,
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
    evidence: &Evidence,
) -> census_domain::model::TeamId {
    let key = format!(
        "{}:{sport:?}:{gender:?}:{}",
        school.as_str(),
        school_year.get()
    );
    teams
        .entry(key)
        .or_insert_with(|| {
            let id = CanonicalTeam::mint(school, sport, gender, school_year);
            CanonicalTeam {
                id,
                school: school.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![evidence.clone()],
                retained_conflicts: Vec::new(),
            }
        })
        .id
        .clone()
}
