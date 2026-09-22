//! Assemble the complete Markdown document from pre-built parts.

use std::collections::HashMap;

use super::format_sections;
use super::md_table;
use super::Seeds;

/// Build the document header section.
fn header(report: &serde_json::Value, store_out: &std::path::Path) -> String {
    let mut doc = String::new();
    doc.push_str("# 10. Measured census — what the pipeline actually collected\n");
    let generated_on = report
        .get("generated_on")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown date");
    let store_out_str = store_out.to_string_lossy();
    doc.push_str(&format!(
        "Generated from `{}` on {}. Every figure below is read ",
        store_out_str, generated_on
    ));
    doc.push_str(
        "from `out/report.json`, `out/athletes.jsonl`, `out/meets.jsonl` or the CSVs in `data/`; research-phase "
    );
    doc.push_str(
        "estimates stay in `synthesis/01-acceptance-answers.md` and are labelled as estimates there. Where the two "
    );
    doc.push_str("disagree, this file is the measurement.\n");
    doc.push('\n');
    doc
}

/// Build the totals section.
fn totals_section(totals: &super::data_loader::Totals) -> String {
    let mut doc = String::new();
    doc.push_str("## Totals\n\n");
    doc.push_str("| metric | measured |\n|---|---|\n");
    doc.push_str(&format_sections::totals_body(totals));
    doc.push('\n');
    doc
}

/// Build the by-state section.
fn state_section(state_rows: &[Vec<String>]) -> String {
    let mut doc = String::new();
    doc.push_str("## By state\n\n");
    doc.push_str(&md_table(
        &[
            "state",
            "schools",
            "athletes",
            "Co2027",
            "boys",
            "girls",
            "multi-source",
            "with coach",
            "with coach email",
        ],
        state_rows,
    ));
    doc.push('\n');
    doc
}

/// Build the class-of-2027-by-sport section.
fn sport_section(sports_rows: &[Vec<String>]) -> String {
    let mut doc = String::new();
    doc.push_str("## Class of 2027 by sport\n\n");
    doc.push_str(&md_table(&["bucket", "athletes"], sports_rows));
    doc.push('\n');
    doc
}

/// Build the Athletic.net identities section.
fn identities_section(
    seeds: &Seeds,
    co2027_len: usize,
    m: &super::compute_metrics::Metrics,
) -> String {
    let mut doc = String::new();
    doc.push_str("## Athletic.net identities obtained without querying Athletic.net\n\n");
    doc.push_str(&format_sections::athletic_net_identities(
        seeds,
        co2027_len,
        m.with_an,
        m.with_ms,
        m.multi_rows,
    ));
    doc
}

/// Build the coach coverage section.
fn coach_section(
    coach_rows: &[Vec<String>],
    recruiting: &[HashMap<String, String>],
    m: &super::compute_metrics::Metrics,
) -> String {
    let mut doc = String::new();
    doc.push_str("## Coach coverage\n\n");
    doc.push_str(&md_table(
        &["state", "coach rows", "with email"],
        coach_rows,
    ));
    doc.push('\n');
    doc.push_str(&format_sections::coach_coverage_bullet(
        recruiting.len(),
        m.rec_coach,
        m.rec_email,
        m.rec_ad,
    ));
    doc
}

/// Build the meets section.
fn meets_section(
    meets_by_state_rows: &[Vec<String>],
    provider_rows: &[Vec<String>],
    provider_count: usize,
) -> String {
    let mut doc = String::new();
    doc.push_str("## Meets by state and provider\n\n");
    doc.push_str(&md_table(&["state", "meets"], meets_by_state_rows));
    doc.push('\n');
    doc.push_str(&format!(
        "{} timer/tenant providers published the meet universe; the largest ten:\n",
        super::fmt_comma(provider_count)
    ));
    doc.push('\n');
    doc.push_str(&md_table(&["provider", "meets"], provider_rows));
    doc.push('\n');
    doc
}

/// Build the measured answers section.
fn measured_section(
    seeds: &Seeds,
    total_co2027: u64,
    total_athletes: u64,
    co2027_len: usize,
    m: &super::compute_metrics::Metrics,
    recruiting_len: usize,
) -> String {
    let mut doc = String::new();
    doc.push_str("## Measured answers to the acceptance questions\n\n");
    doc.push_str("| question | measured answer |\n|---|---|\n");
    doc.push_str(&format_sections::measured_answers(
        seeds,
        &format_sections::MeasuredCounts {
            total_co2027,
            total_athletes,
            co2027_len,
            with_an: m.with_an,
            recruiting_len,
            rec_coach: m.rec_coach,
            rec_email: m.rec_email,
            rec_ad: m.rec_ad,
        },
    ));
    doc.push('\n');
    doc
}

/// Build the footer section.
fn footer() -> String {
    let mut doc = String::new();
    doc.push_str("## Limits and honest gaps\n\n");
    doc.push_str(&format_sections::limits());
    doc.push('\n');
    doc.push_str("## Reproduce\n\n");
    doc.push_str(&format_sections::reproduce());
    doc
}

/// Assemble parameters for the document builder.
pub(super) struct DocParts<'a> {
    pub(super) report: &'a serde_json::Value,
    pub(super) store_out: &'a std::path::Path,
    pub(super) seeds: &'a Seeds,
    pub(super) state_rows: &'a [Vec<String>],
    pub(super) coach_rows: &'a [Vec<String>],
    pub(super) metrics: &'a super::compute_metrics::Metrics,
    pub(super) recruiting: &'a [HashMap<String, String>],
    pub(super) co2027: &'a [HashMap<String, String>],
    pub(super) meets_by_state_rows: &'a [Vec<String>],
    pub(super) provider_rows: &'a [Vec<String>],
    pub(super) provider_count: usize,
    pub(super) sports_rows: &'a [Vec<String>],
    pub(super) totals: &'a super::data_loader::Totals,
}

/// Build the full document string.
#[allow(clippy::too_many_arguments)]
pub(super) fn assemble(parts: &DocParts) -> String {
    let mut doc = String::new();
    doc.push_str(&header(parts.report, parts.store_out));
    doc.push_str(&totals_section(parts.totals));
    doc.push_str(&state_section(parts.state_rows));
    doc.push_str(&sport_section(parts.sports_rows));
    doc.push_str(&identities_section(
        parts.seeds,
        parts.co2027.len(),
        parts.metrics,
    ));
    doc.push_str(&coach_section(
        parts.coach_rows,
        parts.recruiting,
        parts.metrics,
    ));
    doc.push_str(&meets_section(
        parts.meets_by_state_rows,
        parts.provider_rows,
        parts.provider_count,
    ));
    doc.push_str(&measured_section(
        parts.seeds,
        parts.totals.total_co2027,
        parts.totals.total_athletes,
        parts.co2027.len(),
        parts.metrics,
        parts.recruiting.len(),
    ));
    doc.push_str(&footer());
    doc
}
