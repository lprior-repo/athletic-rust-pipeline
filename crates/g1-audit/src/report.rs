//! The printed sections that summarise the whole run: the header, the link inventory, the verdict
//! totals, the page filter, the interstitial markers and the digest/byte mismatches.

use indexmap::IndexMap;

use crate::digests::digest_head;
use crate::pyrepr::fmt_dict;

/// The run's header: the label and how many files each input directory held.
pub(crate) fn print_header(label: &str, raw: usize, evidence: usize, parsed: usize) {
    println!(
        "== {} ==\nraw bodies: {}  evidence records: {}  parsed records: {}",
        label, raw, evidence, parsed,
    );
}

/// The link inventory: one line per key, in the order the report fixed.
pub(crate) fn print_link_inventory(link_totals: &IndexMap<String, usize>) {
    println!("link inventory:");
    for key in &[
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
        if let Some(&value) = link_totals.get(*key) {
            println!("  {:7}  {}", value, key);
        }
    }
}

/// The envelope-count-against-rows tally.
pub(crate) fn print_count_vs_rows(count_vs_rows: &IndexMap<String, usize>) {
    let cvr_items: Vec<_> = count_vs_rows.iter().map(|(k, v)| (k.clone(), *v)).collect();
    println!("count vs rows: {}", fmt_dict(&cvr_items));
}

/// The header of the page-verdict section that follows it.
pub(crate) fn print_verdict_header() {
    println!("page verdicts:");
}

/// Every page verdict, keyed as the report prints them.
pub(crate) fn print_verdict_totals(verdict_totals: &IndexMap<String, usize>) {
    println!("verdict totals:");
    let mut vt_items: Vec<_> = verdict_totals
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    vt_items.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    for (key, value) in &vt_items {
        println!("  {:7}  {}", value, key);
    }
}

/// The sport each retained body was fetched for.
pub(crate) fn print_page_filter(filter_totals: &IndexMap<String, usize>) {
    let pf_items: Vec<_> = filter_totals.iter().map(|(k, v)| (k.clone(), *v)).collect();
    println!(
        "page filter (sport the page was fetched for): {}",
        fmt_dict(&pf_items)
    );
}

/// The interstitial markers found in the retained bodies.
pub(crate) fn print_interstitials(interstitials: &IndexMap<String, usize>) {
    if interstitials.is_empty() {
        println!("interstitial markers: none");
    } else {
        let im_items: Vec<_> = interstitials.iter().map(|(k, v)| (k.clone(), *v)).collect();
        println!("interstitial markers: {}", fmt_dict(&im_items));
    }
}

/// The names that are not their bodies' digests, and the records that misstate a byte count.
pub(crate) fn print_mismatches(
    digest_mismatch: &[(String, String)],
    byte_mismatch: &[(String, String, usize)],
) {
    println!(
        "raw-body sha256 mismatches: {}  evidence-bytes mismatches: {}",
        digest_mismatch.len(),
        byte_mismatch.len()
    );
    for (digest, sha) in digest_mismatch {
        println!(
            "  sha mismatch {} -> {}",
            digest_head(digest),
            digest_head(sha)
        );
    }
    for (digest, claimed, actual) in byte_mismatch {
        println!(
            "  bytes mismatch {} claimed={} actual={}",
            digest_head(digest),
            claimed,
            actual
        );
    }
}
