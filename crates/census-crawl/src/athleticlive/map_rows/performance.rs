use super::super::map::{RowContext, Writer};
use super::Mapped;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalPerformance, Evidence, Grade, Mark, PerformanceIdentity, PerformanceResult,
    RetainedConflict, TimingMethod,
};

pub(super) struct PerformanceFacts {
    pub(super) mark: Mark,
    pub(super) wind_mps: Option<f64>,
    pub(super) place: Option<u16>,
    pub(super) heat: Option<String>,
    pub(super) grade: Grade,
    pub(super) row: usize,
    pub(super) contradiction: Option<String>,
}

pub(super) fn write_performance(
    writer: &mut Writer<'_>,
    context: &RowContext<'_>,
    mapped: &Mapped,
    facts: &PerformanceFacts,
) -> CrawlResult<()> {
    let source_key = format!(
        "{}:row{}:{}",
        context.event_key,
        facts.row,
        mapped.athlete.as_str()
    );
    let mut performance = mint(context, mapped, facts, &source_key)?;
    annotate(&mut performance, context, mapped, facts)?;
    reserve_performances(writer, performance.id.as_str())?;
    writer
        .accumulator
        .performances
        .entry(performance.id.as_str().into())
        .or_insert(performance);
    if facts.contradiction.is_some() {
        writer.stats.rows_conflicting_status =
            writer.stats.rows_conflicting_status.saturating_add(1);
    }
    Ok(())
}

fn mint(
    context: &RowContext<'_>,
    mapped: &Mapped,
    facts: &PerformanceFacts,
    source_key: &str,
) -> CrawlResult<CanonicalPerformance> {
    Ok(CanonicalPerformance::new(
        identity(context, mapped, source_key),
        PerformanceResult {
            team: &mapped.team,
            mark: facts.mark.clone(),
            wind_mps: facts.wind_mps,
            place: facts.place,
        },
    )?)
}

fn identity<'a>(
    context: &'a RowContext<'_>,
    mapped: &'a Mapped,
    source_key: &'a str,
) -> PerformanceIdentity<'a> {
    PerformanceIdentity {
        athlete: &mapped.athlete,
        event: context.event_id,
        meet: &context.meet.id,
        date: &context.meet.date,
        source_key,
    }
}

fn annotate(
    performance: &mut CanonicalPerformance,
    context: &RowContext<'_>,
    mapped: &Mapped,
    facts: &PerformanceFacts,
) -> CrawlResult<()> {
    performance.heat = facts.heat.clone();
    performance.round = context.round.clone();
    performance.timing = Some(TimingMethod::Unknown);
    performance.observed_grade = Some(facts.grade);
    performance.source_athlete = Some(mapped.source_athlete.clone());
    performance
        .evidence
        .try_reserve(1)
        .map_err(|_| resource("athleticlive performance evidence", 1, 1))?;
    performance
        .evidence
        .push(performance_evidence(context, facts));
    attach_conflict(performance, facts)
}

fn attach_conflict(
    performance: &mut CanonicalPerformance,
    facts: &PerformanceFacts,
) -> CrawlResult<()> {
    let Some(detail) = facts.contradiction.as_deref() else {
        return Ok(());
    };
    performance
        .retained_conflicts
        .try_reserve(1)
        .map_err(|_| resource("athleticlive performance conflicts", 1, 1))?;
    performance.retained_conflicts.push(RetainedConflict::new(
        "Published result status",
        performance.id.as_str(),
        &performance.source_key,
        detail,
    ));
    Ok(())
}

fn performance_evidence(context: &RowContext<'_>, facts: &PerformanceFacts) -> Evidence {
    let mut evidence = context.evidence.clone();
    let note =
        format!(
        "{}; {} row {}: grade {} published on the row, read as grade evidence for school year {}",
        context.evidence.note.as_deref().map_or("", |value| value),
        context.event_key, facts.row, facts.grade, context.school_year.get(),
    );
    evidence.note = Some(match facts.contradiction.as_deref() {
        Some(contradiction) => format!("{note}; {contradiction}"),
        None => note,
    });
    evidence
}

fn reserve_performances(writer: &mut Writer<'_>, id: &str) -> CrawlResult<()> {
    if writer.accumulator.performances.contains_key(id) {
        return Ok(());
    }
    let requested = writer
        .accumulator
        .performances
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("athleticlive performances", usize::MAX, 100_000))?;
    if requested > 100_000 {
        return Err(resource("athleticlive performances", requested, 100_000));
    }
    writer
        .accumulator
        .performances
        .try_reserve(1)
        .map_err(|_| resource("athleticlive performances", requested, 100_000))
}

fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}
