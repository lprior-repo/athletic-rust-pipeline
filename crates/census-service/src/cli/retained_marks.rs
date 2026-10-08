use super::Cli;
use anyhow::{Context, Result};
use census_crawl::milesplit::{parse_published_metric_distance, parse_published_time};
use census_domain::model::{
    CanonicalEvent, CanonicalPerformance, CentiMetres, EventId, EventKind, ExactSeconds, Mark,
    SourceNamespace, TimingMethod,
};
use census_store::{Store, StoreError, StoreResult, Table};
use clap::Args;
use std::collections::HashMap;

const WRITE_CHUNK: usize = 100;

#[derive(Debug, Args)]
#[command(
    about = "Normalize retained MileSplit declared timing and metric marks without acquisition; explicit --store required"
)]
pub(super) struct RepairRetainedMarksArgs {
    #[arg(
        long,
        help = "Append corrections; otherwise inspect without changing observations"
    )]
    pub(super) apply: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RepairMode {
    DryRun,
    Apply,
}

enum DeclaredMark {
    Time(ExactSeconds, TimingMethod),
    Metric(CentiMetres),
}

#[derive(Default, Debug)]
struct RepairReport {
    scanned: u64,
    eligible: u64,
    corrected: u64,
    non_numeric_or_undeclared: u64,
    contradictory_timing: u64,
    missing_event: u64,
    missing_evidence: u64,
}

pub(super) fn run_repair_retained_marks(cli: &Cli, args: &RepairRetainedMarksArgs) -> Result<()> {
    let root = cli
        .store
        .as_deref()
        .context("repair-retained-marks requires explicit --store and a stopped store owner")?;
    let store = Store::open(root).context("opening the retained store")?;
    let mode = if args.apply {
        RepairMode::Apply
    } else {
        RepairMode::DryRun
    };
    let report = process_retained_marks(&store, mode).context("normalizing retained marks")?;
    println!(
        "repair-retained-marks\t{}",
        if args.apply { "apply" } else { "dry-run" }
    );
    println!(
        "rows_scanned\t{}\neligible_corrections\t{}\ncorrected\t{}",
        report.scanned, report.eligible, report.corrected
    );
    println!("non_numeric_or_undeclared\t{}\ncontradictory_timing\t{}\nmissing_event\t{}\nmissing_evidence\t{}", report.non_numeric_or_undeclared, report.contradictory_timing, report.missing_event, report.missing_evidence);
    Ok(())
}

fn process_retained_marks(store: &Store, mode: RepairMode) -> StoreResult<RepairReport> {
    let snapshot = store.snapshot();
    let mut events = HashMap::new();
    snapshot.for_each_merged(Table::Events, |event: CanonicalEvent| {
        events
            .try_reserve(1)
            .map_err(|error| allocation(error.to_string()))?;
        events.entry(event.id).or_insert(event.kind);
        Ok(())
    })?;
    let mut report = RepairReport::default();
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(WRITE_CHUNK)
        .map_err(|error| allocation(error.to_string()))?;
    snapshot.for_each_merged(Table::Performances, |perf: CanonicalPerformance| {
        bump(&mut report.scanned)?;
        if let Some(corrected) = correction_for(perf, &events, mode, &mut report)? {
            pending.push(corrected);
            if pending.len() == WRITE_CHUNK {
                append_chunk(store, &mut pending, &mut report)?;
            }
        }
        Ok(())
    })?;
    if mode == RepairMode::Apply {
        append_chunk(store, &mut pending, &mut report)?;
        store.flush()?;
    }
    Ok(report)
}

fn correction_for(
    mut perf: CanonicalPerformance,
    events: &HashMap<EventId, EventKind>,
    mode: RepairMode,
    report: &mut RepairReport,
) -> StoreResult<Option<CanonicalPerformance>> {
    let Some(declared) = eligible_mark(&perf, events, report)? else {
        return Ok(None);
    };
    bump(&mut report.eligible)?;
    if mode == RepairMode::DryRun {
        return Ok(None);
    }
    apply_declared(&mut perf, declared)?;
    Ok(Some(perf))
}

fn eligible_mark(
    perf: &CanonicalPerformance,
    events: &HashMap<EventId, EventKind>,
    report: &mut RepairReport,
) -> StoreResult<Option<DeclaredMark>> {
    let Mark::Raw(raw) = &perf.mark else {
        return Ok(None);
    };
    if !retained_owner(perf) {
        return Ok(None);
    }
    let Some(kind) = events.get(&perf.event) else {
        bump(&mut report.missing_event)?;
        return Ok(None);
    };
    let declared = declared_mark(raw, kind, perf.timing, report)?;
    if declared.is_some() && perf.evidence.is_empty() {
        bump(&mut report.missing_evidence)?;
        return Ok(None);
    }
    Ok(declared)
}

fn retained_owner(perf: &CanonicalPerformance) -> bool {
    perf.source_athlete.as_ref().is_some_and(|source| {
        matches!(&source.namespace, SourceNamespace::Other(name) if name == "milesplit_result_row")
    })
}

fn apply_declared(perf: &mut CanonicalPerformance, declared: DeclaredMark) -> StoreResult<()> {
    let Mark::Raw(raw) = &perf.mark else {
        return Err(correction_invariant("raw token"));
    };
    let evidence = perf
        .evidence
        .first()
        .ok_or_else(|| correction_invariant("source evidence"))?;
    let (mark, timing, note) = declared_values(declared, raw, perf.timing);
    let correction = census_domain::model::Evidence::derived(
        evidence.source.clone(),
        &evidence.observed_on,
        note,
    );
    perf.evidence
        .try_reserve(1)
        .map_err(|error| allocation(error.to_string()))?;
    perf.evidence.push(correction);
    perf.mark = mark;
    perf.timing = timing;
    Ok(())
}

fn correction_invariant(missing: &str) -> StoreError {
    StoreError::Invariant {
        detail: format!("retained mark correction lost its {missing}"),
    }
}

fn declared_values(
    declared: DeclaredMark,
    raw: &str,
    known_timing: Option<TimingMethod>,
) -> (Mark, Option<TimingMethod>, String) {
    match declared {
        DeclaredMark::Time(seconds, timing) => time_values(seconds, raw, timing),
        DeclaredMark::Metric(distance) => (
            Mark::DistanceMetres(distance),
            known_timing,
            format!(
                "milesplit-metric-suffix revision=1 original_raw={raw} centimetres={}",
                distance.value()
            ),
        ),
    }
}

fn time_values(
    seconds: ExactSeconds,
    raw: &str,
    timing: TimingMethod,
) -> (Mark, Option<TimingMethod>, String) {
    (
        Mark::TimeSeconds(seconds),
        Some(timing),
        format!(
            "milesplit-time-suffix revision=2 original_raw={raw} nanoseconds={} timing={}",
            seconds.value(),
            timing.stable_key(),
        ),
    )
}

fn declared_mark(
    raw: &str,
    kind: &EventKind,
    known_timing: Option<TimingMethod>,
    report: &mut RepairReport,
) -> StoreResult<Option<DeclaredMark>> {
    if let Some(distance) = parse_published_metric_distance(raw) {
        return Ok(Some(DeclaredMark::Metric(distance)));
    }
    if kind.is_field() {
        return Ok(None);
    }
    let Some((seconds, Some(timing))) = parse_published_time(raw) else {
        bump(&mut report.non_numeric_or_undeclared)?;
        return Ok(None);
    };
    if known_timing.is_some_and(|known| known != TimingMethod::Unknown && known != timing) {
        bump(&mut report.contradictory_timing)?;
        return Ok(None);
    }
    Ok(Some(DeclaredMark::Time(seconds, timing)))
}

fn append_chunk(
    store: &Store,
    pending: &mut Vec<CanonicalPerformance>,
    report: &mut RepairReport,
) -> StoreResult<()> {
    if pending.is_empty() {
        return Ok(());
    }
    store.append_many(Table::Performances, pending)?;
    report.corrected = report
        .corrected
        .checked_add(u64::try_from(pending.len()).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    pending.clear();
    Ok(())
}

fn bump(counter: &mut u64) -> StoreResult<()> {
    *counter = counter.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    Ok(())
}

fn allocation(detail: String) -> StoreError {
    StoreError::Invariant {
        detail: format!("retained mark correction allocation failed: {detail}"),
    }
}

#[cfg(test)]
#[path = "retained_marks_tests.rs"]
mod tests;
