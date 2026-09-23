//! What one raw body's rows add up to: the link inventory, the count-against-rows comparison, the
//! row-issue tally, the parser join and the class samples.

use std::collections::{BTreeMap, HashSet};

use indexmap::IndexMap;
use serde_json::Value;

use crate::counts::{count_len, tally};
use crate::evidence::EvidenceEntry;
use crate::pages::{Body, PageAnalysis, PageScan};
use crate::parser_join::{self, ParserView};
use crate::rows::{analyse_row, RowAnalysis, RowRegexes};
use crate::samples;

/// One body's rows, each analysed, and the counts the link inventory and the page entry read off them.
struct RowTotals {
    /// One analysis per row, in document order.
    analyses: Vec<RowAnalysis>,
    /// Rows that are same-sport candidates with no issue.
    candidates: usize,
    /// Noncanonical athlete hrefs across the rows.
    noncanonical: usize,
    /// Canonical track-and-field athlete hrefs.
    tf_hrefs: usize,
    /// Canonical cross-country athlete hrefs.
    xc_hrefs: usize,
}

/// The link inventory's keys, in the order the report prints them, all starting at zero.
pub(crate) fn link_total_slots() -> IndexMap<String, usize> {
    let mut link_totals: IndexMap<String, usize> = IndexMap::new();
    for k in &[
        "rows_total",
        "rows_with_athlete_href",
        "rows_without_athlete_href",
        "athlete_hrefs_total",
        "athlete_hrefs_sported_tf",
        "athlete_hrefs_sported_xc",
        "athlete_hrefs_noncanonical",
        "rows_multi_athlete_href",
        "rows_candidate_predicted",
    ] {
        link_totals.insert(k.to_string(), 0);
    }
    link_totals
}

/// Analyse every row of one body.
fn row_totals(rows: &[&str], sport: Option<&str>, re: &RowRegexes<'_>) -> RowTotals {
    let analyses: Vec<RowAnalysis> = rows.iter().map(|r| analyse_row(r, sport, re)).collect();
    let candidates = analyses.iter().filter(|a| a.candidate).count();
    let noncanonical = analyses.iter().map(|a| a.noncanonical.len()).sum();
    let tf_hrefs = analyses
        .iter()
        .flat_map(|a| &a.sported)
        .filter(|s| *s == "tf")
        .count();
    let xc_hrefs = analyses
        .iter()
        .flat_map(|a| &a.sported)
        .filter(|s| *s == "xc")
        .count();
    RowTotals {
        analyses,
        candidates,
        noncanonical,
        tf_hrefs,
        xc_hrefs,
    }
}

/// Add one body's rows and links to the link inventory.
fn add_link_totals(totals: &mut IndexMap<String, usize>, rows_len: usize, rows: &RowTotals) {
    let with_href = rows
        .analyses
        .iter()
        .filter(|a| !a.athlete_hrefs.is_empty())
        .count();
    let without_href = rows
        .analyses
        .iter()
        .filter(|a| a.athlete_hrefs.is_empty())
        .count();
    let hrefs_total: usize = rows.analyses.iter().map(|a| a.athlete_hrefs.len()).sum();
    let multi = rows
        .analyses
        .iter()
        .filter(|a| a.athlete_hrefs.len() > 1)
        .count();
    tally(totals, "rows_total", rows_len);
    tally(totals, "rows_with_athlete_href", with_href);
    tally(totals, "rows_without_athlete_href", without_href);
    tally(totals, "athlete_hrefs_total", hrefs_total);
    tally(totals, "athlete_hrefs_sported_tf", rows.tf_hrefs);
    tally(totals, "athlete_hrefs_sported_xc", rows.xc_hrefs);
    tally(totals, "athlete_hrefs_noncanonical", rows.noncanonical);
    tally(totals, "rows_multi_athlete_href", multi);
    tally(totals, "rows_candidate_predicted", rows.candidates);
}

/// Count where the envelope count sits relative to the page's rows.
fn record_count_vs_rows(
    count: Option<i64>,
    rows_len: usize,
    count_vs_rows: &mut IndexMap<String, usize>,
) {
    if count.is_none() {
        tally(count_vs_rows, "count_absent", 1);
    } else if let Some(c) = count {
        match count_len(c).cmp(&rows_len) {
            std::cmp::Ordering::Equal => tally(count_vs_rows, "count_eq_rows", 1),
            std::cmp::Ordering::Greater => tally(count_vs_rows, "count_gt_rows", 1),
            std::cmp::Ordering::Less => tally(count_vs_rows, "count_lt_rows", 1),
        }
    }
}

/// The row issue messages, counted.
fn row_issue_tally(row_analyses: &[RowAnalysis]) -> IndexMap<String, usize> {
    let mut row_issues_map: IndexMap<String, usize> = IndexMap::new();
    for issue in row_analyses.iter().flat_map(|a| a.issues.iter()) {
        tally(&mut row_issues_map, issue, 1);
    }
    row_issues_map
}

/// The distinct queries the body was fetched for.
fn queries_of(entries: &[EvidenceEntry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|e| e.query.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// One body's line of the per-page table.
fn page_entry(body: &Body<'_>, rows: &RowTotals, parser: ParserView) -> PageAnalysis {
    PageAnalysis {
        digest: body.digest.to_string(),
        sport: body.sport.clone(),
        queries: queries_of(body.entries),
        count: body.count,
        rows: body.rows.len(),
        tf_hrefs: rows.tf_hrefs,
        xc_hrefs: rows.xc_hrefs,
        noncanonical: rows.noncanonical,
        row_issues: row_issue_tally(&rows.analyses),
        candidates: rows.candidates,
        has_next: body.pager.contains("data-start"),
        parser_candidates: parser.candidates,
        parser_issues: parser.issues,
        parser_count: parser.count,
        parser_next: parser.next,
        parser_verdicts: parser.verdicts,
    }
}

/// Keep the samples this body contributes: its noncanonical hrefs, and its over-counted page.
fn push_samples(out: &mut PageScan, body: &Body<'_>, rows: &RowTotals, limit: usize) {
    samples::push_noncanonical(&mut out.class_samples, body.digest, &rows.analyses, limit);
    if let Some(c) = body.count {
        if count_len(c) > body.rows.len() {
            samples::push_count_gt_rows(
                &mut out.class_samples,
                body.digest,
                body.sport.as_deref(),
                c,
                body.rows.len(),
                body.pager,
                limit,
            );
        }
    }
}

/// Analyse one parsed body: its rows, its links, its count comparison, its parser join and samples.
pub(crate) fn analyse_body(
    out: &mut PageScan,
    body: &Body<'_>,
    re: &RowRegexes<'_>,
    parsed_files_map: &BTreeMap<String, Value>,
    samples_limit: usize,
) {
    let rows = row_totals(&body.rows, body.sport.as_deref(), re);
    add_link_totals(&mut out.link_totals, body.rows.len(), &rows);
    record_count_vs_rows(body.count, body.rows.len(), &mut out.count_vs_rows);

    let parser = parser_join::view(body.entries, parsed_files_map);
    out.per_page.push(page_entry(body, &rows, parser));
    push_samples(out, body, &rows, samples_limit);
}
