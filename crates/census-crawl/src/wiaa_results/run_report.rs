use super::{EntityCounts, Stats};
use crate::AdapterReport;

pub(super) fn note_artifacts(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "artifacts: {} seen, {} parsed, {} skipped (unrecognised extension), {} unsupported, {} \
         unparsed-body, {} failed",
        stats.artifacts_seen,
        stats.artifacts_parsed,
        stats.artifacts_unparsed,
        stats.artifacts_unsupported,
        stats.artifacts_parse_failed,
        stats.artifacts_failed
    ));
    report.note(format!(
        "pdf artifacts: {} parsed, {} not a known report layout, {} unreadable (pdftotext)",
        stats.pdf_parsed, stats.pdf_unparsed, stats.pdf_tool_failures
    ));
    report.note(format!("pdf layouts: {:?}", stats.pdf_layouts));
    report.note(format!(
        "formats parsed: {:?}",
        stats
            .formats
            .iter()
            .filter(|(format, _)| format.as_str() != "unparsed")
            .collect::<Vec<_>>()
    ));
    report.note(format!(
        "seasons parsed: {:?}",
        stats.seasons.iter().collect::<Vec<_>>()
    ));
    report.note(format!(
        "result rows: {} (grade-bearing {}), relay legs {}, rows without a school label {}",
        stats.rows, stats.rows_with_grade, stats.relay_legs, stats.rows_without_school
    ));
}

pub(super) fn note_entities(report: &mut AdapterReport, counts: &EntityCounts) {
    report.note(format!(
        "canonical entities: meets {} events {} athletes {} teams {} performances {} relay \
         results {}",
        counts.meets,
        counts.events,
        counts.athletes,
        counts.teams,
        counts.performances,
        counts.relay_results
    ));
    report.note(format!(
        "unsupported cohort observations retained: {}",
        counts.unsupported_cohorts
    ));
}

pub(super) fn note_resolution(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "school label resolution: {}",
        stats
            .school_resolved
            .iter()
            .map(|(kind, count)| format!("{kind}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    if !stats.ambiguous.is_empty() {
        let mut ambiguous: Vec<(&String, &usize)> = stats.ambiguous.iter().collect();
        ambiguous.sort_by(|a, b| b.1.cmp(a.1));
        report.note(format!(
            "ambiguous school labels ({}): {}",
            stats.ambiguous.values().sum::<usize>(),
            ambiguous
                .iter()
                .take(12)
                .map(|(name, count)| format!("{name} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !stats.unresolved.is_empty() {
        let mut unresolved: Vec<(&String, &usize)> = stats.unresolved.iter().collect();
        unresolved.sort_by(|a, b| b.1.cmp(a.1));
        report.note(format!(
            "unresolved school labels ({}): {}",
            stats.unresolved.values().sum::<usize>(),
            unresolved
                .iter()
                .take(12)
                .map(|(name, count)| format!("{name} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}
