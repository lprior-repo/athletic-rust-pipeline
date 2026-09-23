//! Writing one mapped row's performance: the key it is minted under, its mark, and its evidence.
//!
//! Part of [`super`]'s row mapping. The key is the event's own key plus the athlete, so an event
//! document and a live-standings payload publishing one race agree on the performance they mint even
//! though the two payloads number their rows differently.

use super::super::map::{RowContext, Writer};
use super::Mapped;
use census_domain::model::{CanonicalPerformance, Evidence, Grade, Mark, TimingMethod};

/// What one mapped row contributes to its performance.
pub(super) struct PerformanceFacts {
    pub(super) mark: Mark,
    pub(super) wind_mps: Option<f64>,
    pub(super) place: Option<u16>,
    pub(super) heat: Option<String>,
    pub(super) grade: Grade,
    /// The row's position in the payload, quoted in the performance's evidence note.
    pub(super) row: usize,
}

/// Write one row's performance, keyed so both routes that publish the same race agree on its id.
pub(super) fn write_performance(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    mapped: &Mapped,
    facts: &PerformanceFacts,
) {
    // The event's key plus the athlete: an event document's row position and a standings payload's
    // row position are different numbers for the same race, so neither can be part of the key.
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
            // Neither payload states a timing method, and AthleticLIVE is not the timer, so none is
            // claimed.
            timing: Some(TimingMethod::Unknown),
            observed_grade: Some(facts.grade),
            evidence: vec![evidence],
            source_key,
            retained_conflicts: Vec::new(),
        });
}

/// The performance's evidence: the document's, annotated with the row's own grade cell.
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
