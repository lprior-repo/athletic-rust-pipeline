use anyhow::Result;
use census_domain::school_directory::{
    collapse_entries, decide, next_due, ChangeSet, ScheduleLedger, ScheduleSource,
    SchoolDirectoryEntry, YearMonth,
};

use super::read::{self, LaneRead};
use super::report::SourceDecision;
use super::SchoolAddressArgs;

pub(super) struct Corpus {
    pub(super) entries: Vec<SchoolDirectoryEntry>,
    pub(super) rows: usize,
    pub(super) skipped: usize,
    pub(super) notes: usize,
    pub(super) merges: usize,
    pub(super) merge_rows: Vec<String>,
}

pub(super) fn collapse(lanes: &[LaneRead]) -> Corpus {
    let skipped: usize = lanes.iter().map(|lane| lane.report.skipped).sum();
    let notes: usize = lanes.iter().map(|lane| lane.report.notes).sum();
    let rows: Vec<SchoolDirectoryEntry> = lanes
        .iter()
        .flat_map(|lane| lane.entries.iter().cloned())
        .collect();
    let raw = rows.len();
    let collapsed = collapse_entries(rows);
    let merges = collapsed.notes.len();
    let merge_rows: Vec<String> = collapsed
        .notes
        .iter()
        .map(|note| {
            let candidates: Vec<String> = note
                .candidates
                .iter()
                .map(|candidate| candidate.label())
                .collect();
            format!("{} -> {}", note.weak.label(), candidates.join(","))
        })
        .collect();
    Corpus {
        entries: collapsed.entries,
        rows: raw,
        skipped,
        notes,
        merges,
        merge_rows,
    }
}

pub(super) fn diff(
    args: &SchoolAddressArgs,
    entries: &[SchoolDirectoryEntry],
) -> Result<Option<ChangeSet>> {
    let previous = match &args.baseline {
        Some(path) => read::read_baseline(path)?,
        None => None,
    };
    Ok(previous.as_ref().map(|baseline| baseline.diff(entries)))
}

pub(super) fn schedule(
    args: &SchoolAddressArgs,
    lanes: &[LaneRead],
    now: Option<YearMonth>,
) -> Result<(ScheduleLedger, Vec<SourceDecision>)> {
    let mut ledger = match &args.ledger {
        Some(path) => read::read_ledger(path)?,
        None => ScheduleLedger::new(),
    };
    let Some(now) = now else {
        return Ok((ledger, Vec::new()));
    };
    let mut sources: Vec<ScheduleSource> = Vec::new();
    for lane in lanes {
        if !sources.contains(&lane.source) {
            sources.push(lane.source);
        }
    }
    let mut schedule = Vec::new();
    for source in sources {
        match source.cadence() {
            Some(cadence) => {
                let decision = decide(cadence, ledger.last(source), now);
                schedule.push(SourceDecision {
                    source: source.label().to_string(),
                    decision: SourceDecision::label(decision).to_string(),
                    due: next_due(cadence, ledger.last(source), now).to_string(),
                });
                ledger.record(source, now);
            }
            None => schedule.push(SourceDecision {
                source: source.label().to_string(),
                decision: "unscheduled".to_string(),
                due: String::new(),
            }),
        }
    }
    Ok((ledger, schedule))
}
