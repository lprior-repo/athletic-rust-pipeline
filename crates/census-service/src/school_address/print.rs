use census_domain::school_directory::ChangeSet;

use super::pipeline::Corpus;
use super::read::LaneRead;
use super::report::{PhaseReport, SourceDecision};

fn print_lane_summaries(lanes: &[LaneRead]) {
    for lane in lanes {
        let digest: String = lane.report.sha256.chars().take(16).collect();
        println!(
            "  {} {}: {} entries, {} skipped, {} notes [{digest}]",
            lane.report.source,
            lane.report.path,
            lane.report.entries,
            lane.report.skipped,
            lane.report.notes
        );
    }
}

fn print_issues(lanes: &[LaneRead]) {
    for lane in lanes {
        for issue in lane.report.skipped_rows.iter() {
            println!(
                "  skipped {}:{} {} {}",
                lane.report.source, issue.line, issue.field, issue.detail
            );
        }
        for issue in lane.report.note_rows.iter() {
            println!(
                "  noted {}:{} {} {}",
                lane.report.source, issue.line, issue.field, issue.detail
            );
        }
    }
}

fn print_phases(phases: Option<PhaseReport>) {
    let Some(phases) = phases else {
        return;
    };
    if phases.geocode_requested {
        println!(
            "geocoded {} entries ({} skipped, {} empty, {} refused, {} unusable, {} transport)",
            phases.geocoded,
            phases.geocode_skipped,
            phases.geocode_empty,
            phases.geocode_refused,
            phases.geocode_unusable,
            phases.geocode_transport
        );
    }
    if phases.validation_requested {
        println!(
            "validated {} addresses ({} skipped, {} rejected, {} unparsed, {} transport)",
            phases.validated,
            phases.validation_skipped,
            phases.validation_rejected,
            phases.validation_unparsed,
            phases.validation_transport
        );
    }
}

pub(super) fn report(
    lanes: &[LaneRead],
    corpus: &Corpus,
    changes: Option<&ChangeSet>,
    schedule: &[SourceDecision],
    phases: Option<PhaseReport>,
    outputs: &[String],
) {
    println!(
        "read {} rows from {} artifact(s): {} skipped, {} noted",
        corpus.rows,
        lanes.len(),
        corpus.skipped,
        corpus.notes
    );
    print_lane_summaries(lanes);
    println!(
        "collapsed to {} entries with {} merge note(s)",
        corpus.entries.len(),
        corpus.merges
    );
    print_issues(lanes);
    for merge in &corpus.merge_rows {
        println!("  merged {merge}");
    }
    if let Some(changes) = changes {
        println!(
            "changes: {} added, {} removed, {} modified",
            changes.added.len(),
            changes.removed.len(),
            changes.modified.len()
        );
    }
    for decision in schedule {
        if decision.due.is_empty() {
            println!("  {}: {}", decision.source, decision.decision);
        } else {
            println!(
                "  {}: {} (next due {})",
                decision.source, decision.decision, decision.due
            );
        }
    }
    print_phases(phases);
    println!("wrote {}", outputs.join(", "));
}
