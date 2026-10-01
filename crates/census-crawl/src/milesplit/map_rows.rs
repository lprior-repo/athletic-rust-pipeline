use super::{team_for, MeetContext, RowWriter};
use crate::hytek;
use crate::result_file::ParsedRow;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalPerformance, Evidence, GradYear, Grade, ObservedGrade,
    SchoolId, SourceAthleteObservation, SourceIdentity, SourceNamespace, Sport, TeamId, TimingMethod,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

pub(super) fn record_row(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    row_index: usize,
) -> usize {
    bump(&mut writer.stats.rows);
    let Some(shape) = shape_of(writer, context, row) else {
        return 0;
    };
    let Some(school_id) = resolve_school(writer, context, row) else {
        return 0;
    };
    let team_id = team_for(
        &mut writer.accumulated.teams,
        &school_id,
        shape.sport,
        context.event.gender,
        context.school_year,
        context.evidence,
    );
    let Some((athlete_id, source)) = record_athlete(
        writer,
        context,
        &school_id,
        &row.name,
        shape.grade,
        row_index,
        &row.school,
    ) else {
        return 0;
    };
    let athlete = AthleteSubject {
        id: athlete_id,
        source,
    };
    record_performance(writer, context, row, row_index, &shape, &athlete, &team_id);
    1
}

struct RowShape {
    sport: Sport,
    grade: Grade,
}

struct AthleteSubject {
    id: AthleteId,
    source: SourceIdentity,
}

fn shape_of(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
) -> Option<RowShape> {
    let Some(sport) = context.sport else {
        bump(&mut writer.stats.rows_without_sport);
        return None;
    };
    let Some(grade) = row.grade else {
        bump(&mut writer.stats.rows_without_grade);
        return None;
    };
    bump(&mut writer.stats.rows_with_grade);
    if row.school.trim().is_empty() {
        bump(&mut writer.stats.rows_without_school);
        return None;
    }
    if !hytek::looks_like_a_name(&row.name) {
        bump(&mut writer.stats.rows_without_name);
        return None;
    }
    if census_domain::model::normalize_name(&row.name)
        == census_domain::model::normalize_name(&row.school)
    {
        bump(&mut writer.stats.rows_school_named);
        return None;
    }
    Some(RowShape { sport, grade })
}

fn record_performance(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    row_index: usize,
    shape: &RowShape,
    athlete: &AthleteSubject,
    team_id: &TeamId,
) {
    let source_key = performance_key(context, row_index);
    let performance_id = CanonicalPerformance::mint(
        &athlete.id,
        &context.meet.id,
        &context.event.kind,
        &context.meet.date,
        &source_key,
    );
    let evidence = performance_evidence(context, shape.grade, row_index);
    writer
        .accumulated
        .performances
        .entry(performance_id.as_str().to_string())
        .or_insert_with(|| CanonicalPerformance {
            id: performance_id,
            athlete: athlete.id.clone(),
            team: team_id.clone(),
            event: context.event_id.clone(),
            meet: context.meet.id.clone(),
            date: context.meet.date.clone(),
            mark: row.mark.clone(),
            wind_mps: row.wind_mps,
            place: row.place,
            heat: row.heat.clone(),
            round: context.event.round.clone(),
            timing: Some(TimingMethod::Unknown),
            observed_grade: Some(shape.grade),
            evidence: vec![evidence],
            source_key,
            source_athlete: Some(athlete.source.clone()),
            retained_conflicts: Vec::new(),
        });
}

fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}

fn resolve_school(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
) -> Option<SchoolId> {
    let index: &SchoolIndex = writer.index;
    let jurisdiction: UsJurisdiction = context.jurisdiction;
    writer
        .resolved
        .entry(row.school.clone())
        .or_insert_with(|| match index.resolve(jurisdiction, &row.school) {
            Some((id, kind)) => {
                let slot = writer
                    .stats
                    .school_resolved
                    .entry(kind.as_str())
                    .or_default();
                *slot = slot.saturating_add(1);
                Some(id)
            }
            None => {
                let slot = writer
                    .stats
                    .unresolved
                    .entry(row.school.clone())
                    .or_default();
                *slot = slot.saturating_add(1);
                None
            }
        })
        .clone()
}

fn admit_athlete(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    member_name: &str,
    grade: Grade,
    row_index: usize,
    school_name: &str,
) -> Option<(GradYear, ObservedGrade, SourceIdentity)> {
    let source = SourceIdentity::new(
        SourceNamespace::Other("milesplit_result_row".to_string()),
        performance_key(context, row_index),
    );
    let observation = ObservedGrade {
        grade,
        school_year: context.school_year,
        source: context.source.clone(),
    };
    writer
        .accumulated
        .unsupported
        .admit(observation, source, |source| {
            SourceAthleteObservation::new(
                source.namespace,
                source.id,
                performance_key(context, row_index),
                member_name,
                &context.evidence.observed_on,
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
    row_index: usize,
    school_name: &str,
) -> Option<(AthleteId, SourceIdentity)> {
    let (grad_year, observation, source) =
        admit_athlete(writer, context, member_name, grade, row_index, school_name)?;
    let athlete_id = CanonicalAthlete::mint(
        school_id,
        member_name,
        grad_year,
        context.event.gender,
        &source,
    );
    let entry = writer
        .accumulated
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
            if let Some(sport) = context.sport {
                athlete.sports.push(sport);
            }
            athlete
        });
    if let Some(sport) = context.sport {
        if !entry.sports.contains(&sport) {
            entry.sports.push(sport);
        }
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

fn performance_key(context: &MeetContext<'_>, row_index: usize) -> String {
    format!("{}:{}:{}", context.rsid, context.event.label, row_index)
}

fn performance_evidence(context: &MeetContext<'_>, grade: Grade, row_index: usize) -> Evidence {
    let mut evidence = context.evidence.clone();
    evidence.note = Some(format!(
        "RSID {} row {row_index}: Yr {} published on the row, read as grade evidence for school \
         year {}",
        context.rsid,
        grade.get(),
        context.school_year.get()
    ));
    evidence
}
