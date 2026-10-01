use super::{performance_evidence, MeetContext, RowWriter};
use crate::result_file::ParsedRow;
use census_domain::model::{AthleteId, CanonicalPerformance, Grade, SourceIdentity, TeamId};

pub(super) struct MemberFacts {
    pub(super) athlete: AthleteId,
    pub(super) source: SourceIdentity,
    pub(super) key: String,
    pub(super) grade: Grade,
    pub(super) leg_position: Option<u8>,
}

pub(super) fn record_performance(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &ParsedRow,
    team: &TeamId,
    facts: MemberFacts,
) {
    let id = CanonicalPerformance::mint(
        &facts.athlete,
        &context.meet.id,
        &context.event.kind,
        &context.meet.date,
        &facts.key,
    );
    writer
        .accumulator
        .performances
        .entry(id.as_str().to_string())
        .or_insert_with(|| CanonicalPerformance {
            id,
            athlete: facts.athlete,
            team: team.clone(),
            event: context.event_id.clone(),
            meet: context.meet.id.clone(),
            date: context.meet.date.clone(),
            mark: row.mark.clone(),
            wind_mps: row.wind_mps,
            place: row.place,
            heat: row.heat.clone(),
            round: context.event.round.clone(),
            timing: Some(context.timing),
            observed_grade: Some(facts.grade),
            evidence: vec![performance_evidence(context, row, facts.leg_position)],
            source_key: facts.key,
            source_athlete: Some(facts.source),
            retained_conflicts: Vec::new(),
        });
}
