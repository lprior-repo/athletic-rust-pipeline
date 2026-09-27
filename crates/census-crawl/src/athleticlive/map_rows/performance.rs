use super::super::map::{RowContext, Writer};
use super::Mapped;
use census_domain::model::{CanonicalPerformance, Evidence, Grade, Mark, TimingMethod};

pub(super) struct PerformanceFacts {
    pub(super) mark: Mark,
    pub(super) wind_mps: Option<f64>,
    pub(super) place: Option<u16>,
    pub(super) heat: Option<String>,
    pub(super) grade: Grade,
    pub(super) row: usize,
}

pub(super) fn write_performance(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    mapped: &Mapped,
    facts: &PerformanceFacts,
) {
    let source_key = format!("{}:{}", context.event_key, mapped.athlete.as_str());
    let performance_id = CanonicalPerformance::mint(
        &mapped.athlete,
        &context.meet.id,
        context.kind,
        &context.meet.date,
        &source_key,
    );
    let evidence = performance_evidence(context, facts);
    writer
        .accumulator
        .performances
        .entry(performance_id.as_str().to_string())
        .or_insert_with(|| CanonicalPerformance {
            id: performance_id,
            athlete: mapped.athlete.clone(),
            team: mapped.team.clone(),
            event: context.event_id.clone(),
            meet: context.meet.id.clone(),
            date: context.meet.date.clone(),
            mark: facts.mark.clone(),
            wind_mps: facts.wind_mps,
            place: facts.place,
            heat: facts.heat.clone(),
            round: context.round.clone(),
            timing: Some(TimingMethod::Unknown),
            observed_grade: Some(facts.grade),
            evidence: vec![evidence],
            source_key,
            source_athlete: mapped.source_athlete.clone(),
            retained_conflicts: Vec::new(),
        });
}

fn performance_evidence(context: &RowContext<'_>, facts: &PerformanceFacts) -> Evidence {
    let mut evidence = context.evidence.clone();
    evidence.note = Some(format!(
        "{} row {}: grade {} published on the row, read as grade evidence for school year {}",
        context.event_key,
        facts.row,
        facts.grade,
        context.school_year.get()
    ));
    evidence
}
