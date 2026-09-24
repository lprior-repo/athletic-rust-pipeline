//! The page-level tallies: what the bytes say about each page, what the parser says about the same
//! bytes, and the verdict totals those two readings add up to.

use indexmap::IndexMap;

use crate::counts::{count_len, len_count, tally};
use crate::pages::PageAnalysis;

/// The page's parser verdicts, sorted and deduplicated.
fn distinct_verdicts(page: &PageAnalysis) -> Vec<String> {
    let mut v = page.parser_verdicts.clone();
    v.sort();
    v.dedup();
    v
}

/// Whether the page carries any row issue.
fn has_row_issues(page: &PageAnalysis) -> bool {
    page.row_issues.values().sum::<usize>() > 0
}

/// Whether a page's distinct parser verdicts are exactly `only`.
fn verdicts_are(page: &PageAnalysis, only: &str) -> bool {
    distinct_verdicts(page) == [only.to_string()]
}

/// Whether any of the page's parser verdicts is `verdict`.
fn has_verdict(page: &PageAnalysis, verdict: &str) -> bool {
    page.parser_verdicts.iter().any(|v| v == verdict)
}

/// Pages whose parser record reported issues.
fn parser_issue_pages(per_page: &[PageAnalysis]) -> usize {
    per_page
        .iter()
        .filter(|p| !p.parser_issues.is_empty())
        .count()
}

/// The bytes-only recount: page kinds as the rows and the envelope's own pager classify them.
pub(crate) fn print_recount(per_page: &[PageAnalysis]) {
    let pages_carrying_candidate_rows: usize = per_page.iter().filter(|p| p.candidates > 0).count();
    let row_issue_pages: usize = per_page.iter().filter(|p| has_row_issues(p)).count();
    let short_pages: usize = per_page
        .iter()
        .filter(|p| {
            p.count
                .map(|c| count_len(c) > p.rows && !p.has_next)
                .unwrap_or(false)
        })
        .count();
    let count_lt_rows: usize = per_page
        .iter()
        .filter(|p| p.count.map(|c| count_len(c) < p.rows).unwrap_or(false))
        .count();
    let clean_full: usize = per_page
        .iter()
        .filter(|p| p.count.map(|c| count_len(c) == p.rows).unwrap_or(false) && !has_row_issues(p))
        .count();

    println!("  recount (bytes only):");
    println!(
        "    pages={} row_issue_pages={} short_page={} count_lt_rows={} clean_full={}",
        per_page.len(),
        row_issue_pages,
        short_pages,
        count_lt_rows,
        clean_full
    );
    let candidate_rows_lost: usize = per_page
        .iter()
        .filter(|p| has_row_issues(p))
        .map(|p| p.candidates)
        .sum();
    println!(
        "    pages_carrying_candidate_rows={} of_which_row_issue_pages={} candidate_rows_lost_on_those_pages={}",
        pages_carrying_candidate_rows,
        per_page
            .iter()
            .filter(|p| p.candidates > 0 && has_row_issues(p))
            .count(),
        candidate_rows_lost
    );
}

/// The same page kinds as the parser's own records classify them.
pub(crate) fn print_parser(per_page: &[PageAnalysis]) {
    let parser_short_pages: usize = per_page
        .iter()
        .filter(|p| has_verdict(p, "short_page"))
        .count();
    let parser_clean_pages: usize = per_page
        .iter()
        .filter(|p| verdicts_are(p, "clean_full"))
        .count();
    let parser_cand_pages: usize = per_page
        .iter()
        .filter(|p| (p.parser_candidates.unwrap_or(0)) > 0)
        .count();
    let parser_salvageable: usize = per_page
        .iter()
        .filter(|p| (p.parser_candidates.unwrap_or(0)) > 0 && !p.parser_issues.is_empty())
        .count();
    let no_parsed_record: usize = per_page
        .iter()
        .filter(|p| distinct_verdicts(p).is_empty())
        .count();

    println!("  parser (its own records for these same bytes):");
    println!(
        "    pages={} row_issue_pages={} short_page={} clean_full={} no_parsed_record={}",
        per_page.len(),
        parser_issue_pages(per_page),
        parser_short_pages,
        parser_clean_pages,
        no_parsed_record
    );
    println!(
        "    pages_with_candidates={} of_which_row_issue_pages={} candidate_rows_on_those_pages={}",
        parser_cand_pages,
        parser_salvageable,
        per_page
            .iter()
            .filter(|p| (p.parser_candidates.unwrap_or(0)) > 0 && !p.parser_issues.is_empty())
            .map(|p| p.parser_candidates.unwrap_or(0))
            .sum::<usize>()
    );
}

/// Add the parser-verdict tallies and the row-issue tallies to the verdict map.
pub(crate) fn populate_totals(
    per_page: &[PageAnalysis],
    verdict_totals: &mut IndexMap<String, usize>,
) {
    let clean_pages: usize = per_page
        .iter()
        .filter(|p| has_verdict(p, "clean_full"))
        .count();
    let paginated_pages: usize = per_page
        .iter()
        .filter(|p| has_verdict(p, "paginated"))
        .count();
    let short_pages: usize = per_page
        .iter()
        .filter(|p| has_verdict(p, "short_page"))
        .count();
    let issue_pages = parser_issue_pages(per_page);
    for (key, pages) in [
        ("parser_page:clean_full", clean_pages),
        ("parser_page:paginated", paginated_pages),
        ("parser_page:short_page", short_pages),
        ("parser_page:row_issues", issue_pages),
    ] {
        if pages > 0 {
            tally(verdict_totals, key, pages);
        }
    }
    for page in per_page {
        for (issue_msg, count) in &page.row_issues {
            let key = format!("row_issue:{}", issue_msg);
            tally(verdict_totals, &key, *count);
        }
        // ambiguous_parser_verdict: pages where parser_verdicts has multiple distinct verdicts
        if distinct_verdicts(page).len() > 1 {
            tally(verdict_totals, "ambiguous_parser_verdict", 1);
        }
    }
}

/// The issue composition of the pages the parser flagged.
pub(crate) fn print_issue_classes(per_page: &[PageAnalysis]) {
    let only_noncanonical: usize = per_page
        .iter()
        .filter(|p| {
            p.parser_issues.len() == 1
                && p.parser_issues.contains_key("athlete URL is not canonical")
        })
        .count();
    let only_other: usize = per_page
        .iter()
        .filter(|p| {
            p.parser_issues.len() == 1
                && p.parser_issues
                    .contains_key("result row belongs to another or unspecified sport")
        })
        .count();
    let both_or_other: usize = per_page
        .iter()
        .filter(|p| {
            p.parser_issues.len() > 1
                || (p.parser_issues.len() == 1
                    && !p.parser_issues.contains_key("athlete URL is not canonical")
                    && !p
                        .parser_issues
                        .contains_key("result row belongs to another or unspecified sport"))
        })
        .count();
    println!("  issue classes on row-issue pages:");
    println!("    only_non_canonical_url={}", only_noncanonical);
    println!("    only_other_or_unspecified_sport={}", only_other);
    println!("    both_or_other={}", both_or_other);
}

/// Pages carrying rows of both sports in one response.
pub(crate) fn print_co_mingled(per_page: &[PageAnalysis]) {
    let co_mingled: usize = per_page
        .iter()
        .filter(|p| p.tf_hrefs > 0 && p.xc_hrefs > 0)
        .count();
    let co_mingled_with_issues: usize = per_page
        .iter()
        .filter(|p| p.tf_hrefs > 0 && p.xc_hrefs > 0 && !p.row_issues.is_empty())
        .count();
    let co_mingled_candidates: usize = per_page
        .iter()
        .filter(|p| p.tf_hrefs > 0 && p.xc_hrefs > 0)
        .map(|p| p.candidates)
        .sum();
    println!("  pages carrying rows of both sports in one response (tf hrefs and xc hrefs):");
    println!(
        "    co_mingled_pages={} with_row_issues={} same_sport_candidate_rows_on_them={}",
        co_mingled, co_mingled_with_issues, co_mingled_candidates
    );
}

/// Mid-pagination versus last-page-and-short, over the pages the parser flagged.
pub(crate) fn print_pagination(per_page: &[PageAnalysis]) {
    let with_next: usize = per_page
        .iter()
        .filter(|p| !p.parser_issues.is_empty() && p.parser_next.is_some())
        .count();
    let last_page_short: usize = per_page
        .iter()
        .filter(|p| {
            !p.parser_issues.is_empty()
                && p.parser_next.is_none()
                && (p.parser_count.unwrap_or(0)) > len_count(p.parser_candidates.unwrap_or(0))
        })
        .count();
    println!("  if other-sport rows became skips instead of issues:");
    println!(
        "    row_issue_pages_that_are_mid_pagination(next present)={} row_issue_pages_that_are_last_page_and_short={}",
        with_next, last_page_short
    );
}
