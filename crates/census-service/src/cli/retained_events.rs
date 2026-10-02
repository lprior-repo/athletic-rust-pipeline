use super::Cli;
use anyhow::{Context, Result};
use census_domain::model::{CanonicalEvent, EventKind, Evidence, EvidenceMethod, SourceRef};
use census_store::{Store, StoreError, StoreResult, Table};
use clap::Args;

const WRITE_CHUNK: usize = 100;

#[derive(Debug, Args)]
#[command(
    about = "Resolve source-proven retained event labels without acquisition; explicit --store and a stopped store owner required"
)]
pub(super) struct RepairRetainedEventsArgs {
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

#[derive(Default, Debug)]
struct RepairReport {
    scanned: u64,
    unmapped: u64,
    eligible: u64,
    corrected: u64,
    unsupported_or_unbound: u64,
}

pub(super) fn run_repair_retained_events(cli: &Cli, args: &RepairRetainedEventsArgs) -> Result<()> {
    let root = cli
        .store
        .as_deref()
        .context("repair-retained-events requires explicit --store and a stopped store owner")?;
    let store = Store::open(root).context("opening the retained store; stop its owner first")?;
    let mode = if args.apply {
        RepairMode::Apply
    } else {
        RepairMode::DryRun
    };
    let report =
        process_retained_events(&store, mode).context("resolving retained event labels")?;
    println!(
        "repair-retained-events\t{}",
        if args.apply { "apply" } else { "dry-run" }
    );
    println!(
        "scanned\t{}\nunmapped\t{}\neligible\t{}\ncorrected\t{}\nunsupported_or_unbound\t{}",
        report.scanned,
        report.unmapped,
        report.eligible,
        report.corrected,
        report.unsupported_or_unbound
    );
    Ok(())
}

fn process_retained_events(store: &Store, mode: RepairMode) -> StoreResult<RepairReport> {
    let snapshot = store.snapshot();
    let mut report = RepairReport::default();
    let mut pending = Vec::new();
    if mode == RepairMode::Apply {
        pending
            .try_reserve_exact(WRITE_CHUNK)
            .map_err(|error| allocation(error.to_string()))?;
    }
    snapshot.for_each_merged(Table::Events, |event: CanonicalEvent| {
        bump(&mut report.scanned)?;
        if let Some(corrected) = correction_for(event, mode, &mut report)? {
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
    mut event: CanonicalEvent,
    mode: RepairMode,
    report: &mut RepairReport,
) -> StoreResult<Option<CanonicalEvent>> {
    let EventKind::Unmapped { label } = &event.kind else {
        return Ok(None);
    };
    bump(&mut report.unmapped)?;
    let Some(kind) = event.resolved_source_kind() else {
        bump(&mut report.unsupported_or_unbound)?;
        return Ok(None);
    };
    bump(&mut report.eligible)?;
    if mode == RepairMode::DryRun {
        return Ok(None);
    }
    let source_label = event
        .source_labels
        .iter()
        .find(|source| source.label == *label)
        .ok_or_else(|| invariant("resolved event lost its original source label"))?;
    let evidence = event
        .evidence
        .iter()
        .find(|row| row.method == EvidenceMethod::Parsed && row.source == source_label.source)
        .ok_or_else(|| invariant("resolved event lost its bound parsed evidence"))?;
    let note = correction_note(label, &kind)?;
    let correction = correction_evidence(evidence, note)?;
    event
        .evidence
        .try_reserve(1)
        .map_err(|error| allocation(error.to_string()))?;
    event.evidence.push(correction);
    event.kind = kind;
    Ok(Some(event))
}

fn correction_evidence(evidence: &Evidence, note: String) -> StoreResult<Evidence> {
    let source = SourceRef::new(
        copy_text(&evidence.source.id)?,
        evidence.source.url.as_deref().map(copy_text).transpose()?,
    );
    Ok(Evidence::derived(
        source,
        copy_text(&evidence.observed_on)?,
        note,
    ))
}

fn copy_text(value: &str) -> StoreResult<String> {
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|error| allocation(error.to_string()))?;
    text.push_str(value);
    Ok(text)
}

fn correction_note(label: &str, kind: &EventKind) -> StoreResult<String> {
    let key = kind.stable_key();
    let parts = [
        "retained-event-kind revision=1 original_label=",
        label,
        " kind=",
        key.as_ref(),
    ];
    let size = parts.iter().try_fold(0_usize, |size, part| {
        size.checked_add(part.len())
            .ok_or(StoreError::CounterOverflow)
    })?;
    let mut note = String::new();
    note.try_reserve_exact(size)
        .map_err(|error| allocation(error.to_string()))?;
    parts.iter().for_each(|part| note.push_str(part));
    Ok(note)
}

fn append_chunk(
    store: &Store,
    pending: &mut Vec<CanonicalEvent>,
    report: &mut RepairReport,
) -> StoreResult<()> {
    if pending.is_empty() {
        return Ok(());
    }
    let corrected = report
        .corrected
        .checked_add(u64::try_from(pending.len()).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    store.append_many(Table::Events, pending)?;
    report.corrected = corrected;
    pending.clear();
    Ok(())
}

fn bump(counter: &mut u64) -> StoreResult<()> {
    *counter = counter.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    Ok(())
}

fn allocation(detail: String) -> StoreError {
    invariant(&format!("correction allocation failed: {detail}"))
}

fn invariant(detail: &str) -> StoreError {
    StoreError::Invariant {
        detail: format!("retained event repair: {detail}"),
    }
}

#[cfg(test)]
#[path = "retained_events_tests.rs"]
mod tests;
