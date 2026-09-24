//! The per-page table: every retained body on one line.

use crate::digests::digest_head;
use crate::pages::PageAnalysis;

/// The queries the page was fetched for, sorted, joined, and cut at 60 bytes.
fn queries_field(queries: &[String]) -> String {
    let mut queries_sorted: Vec<&str> = queries.iter().map(|s| &**s).collect();
    queries_sorted.sort();
    let queries_str = queries_sorted.join(",");
    queries_str
        .split_at_checked(60)
        .map_or(queries_str.as_str(), |(head, _)| head)
        .to_string()
}

/// One page's line: digest, filter, count, rows, href classes, issues and queries.
fn print_page(page: &PageAnalysis) {
    let digest = &page.digest;
    let sport_fmt = format!("{:3}", page.sport.as_deref().unwrap_or("None"));

    let count_str = match page.count {
        Some(n) => n.to_string(),
        None => "None".to_string(),
    };
    let count_fmt = if count_str.len() <= 5 {
        format!("{:>5}", count_str)
    } else {
        count_str
    };

    let issues_str: Vec<_> = page
        .row_issues
        .iter()
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect();
    let parser_issues_str: Vec<_> = page
        .parser_issues
        .iter()
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect();

    let parser_cand = page.parser_candidates.unwrap_or(0);
    let queries_trunc = queries_field(&page.queries);
    println!(
        "  {} {} count={} rows={:3} tf={:3} xc={:3} noncanon={:2} next={} pred_issues=[{}] cand={} parser_cand={} parser_issues=[{}] q={}",
        digest_head(digest),
        sport_fmt,
        count_fmt,
        page.rows,
        page.tf_hrefs,
        page.xc_hrefs,
        page.noncanonical,
        if page.has_next { "y" } else { "n" },
        issues_str.join(","),
        page.candidates,
        parser_cand,
        parser_issues_str.join(","),
        queries_trunc,
    );
}

/// The header plus one line per retained body, in directory order.
pub(crate) fn print(per_page: &[PageAnalysis]) {
    println!(
        "per-page (digest, filter, count, rows, tf, xc, noncanon, predicted issues, candidates, parser candidates/issues):"
    );
    for page in per_page {
        print_page(page);
    }
}
