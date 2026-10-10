#[path = "map_rows/performance.rs"]
mod performance;

use super::super::Stats;
use super::{team_for, ArchiveArtifact, MeetContext, RowWriter};
use crate::result_file::{ParsedEvent, ParsedRow};
use census_domain::model::{
    AthleteId, CanonicalAthlete, GradYear, Grade, ObservedGrade, RelayMember, RelayResult,
    SchoolId, SourceAthleteObservation, SourceIdentity, SourceNamespace, SourceRef, TeamId,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use performance::{record_performance, MemberFacts};
use std::collections::HashMap;

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

    let team_id = team_for(
        &mut writer.accumulator.teams,
        &school_id,
        context.sport,
        context.event.gender,
        context.school_year,
        context.evidence,
    );
    if row.legs.is_empty() {
        let Some(grade) = row.grade else {
            return 0;
        };
        if row.name.trim().is_empty() {
            return 0;
        }
        let key = source_key(context.artifact, context.event, row_index, None);
        let Some((athlete_id, source_athlete)) = record_athlete(
            writer,
            context,
            &school_id,
            &row.name,
            grade,
            &key,
            &row.school,
        ) else {
            return 0;
        };
        record_performance(
            writer,
            context,
            row,
            &team_id,
            MemberFacts {
                athlete: athlete_id,
                source: source_athlete,
                key,
                grade,
            },
        );
        return 1;
    }
    writer.stats.relay_legs = writer.stats.relay_legs.saturating_add(row.legs.len());
    record_relay_team(writer, context, row, row_index, &school_id, &team_id)
}

fn record_relay_team(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    row_index: usize,
    school_id: &SchoolId,
    team_id: &TeamId,
) -> usize {
    let mut relay = RelayResult::new(
        team_id,
        context.event_id,
        &context.meet.id,
        row.mark.clone(),
        source_key(context.artifact, context.event, row_index, None),
    )
    .with_place(row.place)
    .with_round(context.event.round.clone())
    .with_timing(row.timing.or(Some(context.timing)))
    .with_evidence(context.evidence.clone());
    let mut athlete_rows = 0usize;
    for leg in &row.legs {
        let mut member = RelayMember::new(u32::from(leg.position), leg.name.clone());
        if leg.name.trim().is_empty() {
            relay = relay.with_member(member);
            continue;
        }
        let leg_key = source_key(
            context.artifact,
            context.event,
            row_index,
            Some(leg.position),
        );
        if let Some(grade) = leg.grade {
            if let Some((athlete_id, _)) = record_athlete(
                writer,
                context,
                school_id,
                &leg.name,
                grade,
                &leg_key,
                &row.school,
            ) {
                athlete_rows = athlete_rows.saturating_add(1);
                member = member.with_athlete(athlete_id);
            }
        }
        relay = relay.with_member(member);
    }
    writer
        .accumulator
        .relay_results
        .entry(relay.source_key.clone())
        .or_insert(relay);
    athlete_rows
}

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

fn admit_athlete(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    member_name: &str,
    grade: Grade,
    source_key: &str,
    school_name: &str,
) -> Option<(GradYear, ObservedGrade, SourceIdentity)> {
    let source = SourceIdentity::new(
        SourceNamespace::Other("wiaa_result_row".to_string()),
        source_key,
    );
    let observation = ObservedGrade {
        grade,
        school_year: context.school_year,
        source: SourceRef::new("wiaa_results", Some(context.artifact.url.clone())),
    };
    writer
        .accumulator
        .unsupported
        .admit(observation, source, |source| {
            SourceAthleteObservation::new(
                source.namespace,
                source.id,
                source_key,
                member_name,
                context.observed_on,
            )
            .with_school(Some(school_name.into()))
            .with_gender(context.event.gender)
        })
}

fn record_athlete(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    school_id: &SchoolId,
    member_name: &str,
    grade: Grade,
    source_key: &str,
    school_name: &str,
) -> Option<(AthleteId, SourceIdentity)> {
    let (grad_year, observation, source) =
        admit_athlete(writer, context, member_name, grade, source_key, school_name)?;
    let athlete_id = CanonicalAthlete::mint(
        school_id,
        member_name,
        grad_year,
        context.event.gender,
        &source,
    );
    let entry = writer
        .accumulator
        .athletes
        .entry(athlete_id.as_str().to_string())
        .or_insert_with(|| {
            let mut athlete = CanonicalAthlete::new(
                school_id,
                member_name,
                grad_year,
                context.event.gender,
                source.clone(),
            );
            athlete.sports.push(context.sport);
            athlete
        });
    if !entry.sports.contains(&context.sport) {
        entry.sports.push(context.sport);
    }
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
    Some((athlete_id, source))
}

fn round_label(round: &Option<String>) -> &str {
    match round {
        Some(label) => label,
        None => "<none>",
    }
}

pub(super) fn source_key(
    artifact: &ArchiveArtifact,
    event: &ParsedEvent,
    row_index: usize,
    leg_position: Option<u8>,
) -> String {
    match leg_position {
        Some(position) => format!(
            "{}:{}:{}:{}:leg{}",
            artifact.url,
            event.label,
            round_label(&event.round),
            row_index,
            position
        ),
        None => format!(
            "{}:{}:{}:{}",
            artifact.url,
            event.label,
            round_label(&event.round),
            row_index
        ),
    }
}
