//! The raw-body scan: read each retained body, decide what it is, and hand its rows to the census.

use std::collections::{BTreeMap, HashSet};
use std::fs;

use indexmap::IndexMap;
use serde_json::Value;

use crate::counts::bump;
use crate::digests::sha256_hex;
use crate::evidence::EvidenceEntry;
use crate::rowcensus;
use crate::rows::Regexes;
use crate::samples::Samples;

/// Markers that mean the body is a challenge or block page rather than a search response.
static INTERSTITIAL_MARKERS: &[&str] = &[
    "just a moment",
    "attention required",
    "cf-challenge",
    "challenge-platform",
    "cf-ray",
    "captcha",
    "turnstile",
    "hcaptcha",
    "recaptcha",
    "pardon our interruption",
    "access denied",
    "request blocked",
    "enable javascript and cookies",
    "verify you are human",
    "unusual traffic",
];

/// One response body's rows, links and verdicts, as the per-page table prints them.
pub(crate) struct PageAnalysis {
    /// The body's digest (its file stem in the raw directory).
    pub(crate) digest: String,
    /// The sport the body was fetched for, when the evidence records agree on one.
    pub(crate) sport: Option<String>,
    /// The queries that fetched it.
    pub(crate) queries: Vec<String>,
    /// The envelope's `count`, as the site reported it.
    pub(crate) count: Option<i64>,
    /// Rows found by splitting `results` on `<tr`.
    pub(crate) rows: usize,
    /// Canonical track-and-field athlete hrefs.
    pub(crate) tf_hrefs: usize,
    /// Canonical cross-country athlete hrefs.
    pub(crate) xc_hrefs: usize,
    /// Noncanonical athlete hrefs.
    pub(crate) noncanonical: usize,
    /// Row issue messages, counted.
    pub(crate) row_issues: IndexMap<String, usize>,
    /// Same-sport candidate rows.
    pub(crate) candidates: usize,
    /// Whether the pager carried a next page.
    pub(crate) has_next: bool,
    /// Candidates the parser recorded, when it recorded a candidate list.
    pub(crate) parser_candidates: Option<usize>,
    /// The parser's issue messages for these bytes, counted.
    pub(crate) parser_issues: IndexMap<String, usize>,
    /// The parser's envelope count.
    pub(crate) parser_count: Option<i64>,
    /// The parser's next offset.
    pub(crate) parser_next: Option<i64>,
    /// One verdict per parser record joined to these bytes.
    pub(crate) parser_verdicts: Vec<String>,
}

/// Everything one pass over the raw bodies produces.
#[derive(Default)]
pub(crate) struct PageScan {
    /// One entry per raw body, in directory order.
    pub(crate) per_page: Vec<PageAnalysis>,
    /// The link inventory, pre-seeded with every key the report prints.
    pub(crate) link_totals: IndexMap<String, usize>,
    /// Envelope count against row count, counted.
    pub(crate) count_vs_rows: IndexMap<String, usize>,
    /// Interstitial markers found, counted.
    pub(crate) interstitials: IndexMap<String, usize>,
    /// The sport each page was fetched for, counted.
    pub(crate) filter_totals: IndexMap<String, usize>,
    /// `(name, sha256)` for every body whose name is not its digest.
    pub(crate) digest_mismatch: Vec<(String, String)>,
    /// `(digest, claimed, actual)` for every evidence record that misstates a byte count.
    pub(crate) byte_mismatch: Vec<(String, String, usize)>,
    /// The class samples the report prints.
    pub(crate) class_samples: Samples,
}

/// One raw body, ready for row analysis.
pub(crate) struct Body<'a> {
    /// The body's digest.
    pub(crate) digest: &'a str,
    /// The sport its evidence records agree on.
    pub(crate) sport: &'a Option<String>,
    /// The rows its `results` split into.
    pub(crate) rows: Vec<&'a str>,
    /// The envelope's count.
    pub(crate) count: Option<i64>,
    /// The envelope's pager markup.
    pub(crate) pager: &'a str,
    /// The evidence entries joined to these bytes.
    pub(crate) entries: &'a [EvidenceEntry],
}

/// Record a body whose name is not the digest of its bytes.
fn record_digest_mismatch(digest: &str, blob: &[u8], out: &mut Vec<(String, String)>) {
    let sha = sha256_hex(blob);
    if sha != *digest {
        out.push((digest.to_string(), sha));
    }
}

/// Record every evidence record that misstates this body's byte count.
fn record_byte_mismatch(
    digest: &str,
    entries: &[EvidenceEntry],
    blob_len: usize,
    out: &mut Vec<(String, String, usize)>,
) {
    for entry in entries {
        if let Some(claimed) = &entry.bytes {
            if let Ok(claimed_bytes) = claimed.parse::<usize>() {
                if claimed_bytes != blob_len {
                    out.push((digest.to_string(), claimed.clone(), blob_len));
                }
            }
        }
    }
}

/// The sport a body was fetched for: the single sport its evidence records agree on.
fn sport_of(entries: &[EvidenceEntry]) -> Option<String> {
    let sports: HashSet<&str> = entries.iter().filter_map(|e| e.sport.as_deref()).collect();
    if sports.len() == 1 {
        sports.iter().next().map(|token| match *token {
            "track_field" => "tf".to_string(),
            "cross_country" => "xc".to_string(),
            s => s.to_string(),
        })
    } else {
        None
    }
}

/// Count the markers that say this body is a challenge or block page.
fn record_interstitials(text: &str, interstitials: &mut IndexMap<String, usize>) {
    let lower = text.to_lowercase();
    for marker in INTERSTITIAL_MARKERS {
        if lower.contains(marker) {
            bump(interstitials.entry(marker.to_string()).or_insert(0), 1);
        }
    }
}

/// Parse the response envelope, or count the page as unparseable.
fn envelope(text: &str, verdict_totals: &mut IndexMap<String, usize>) -> Option<Value> {
    match serde_json::from_str(text) {
        Ok(v) => Some(v),
        Err(_) => {
            bump(
                verdict_totals
                    .entry("envelope_unparseable".to_string())
                    .or_insert(0),
                1,
            );
            None
        }
    }
}

/// The `d` object's `results`, `count` and `pager`, or count the page as missing its envelope.
fn envelope_parts<'a>(
    envelope: &'a Value,
    verdict_totals: &mut IndexMap<String, usize>,
) -> Option<(&'a str, Option<i64>, &'a str)> {
    // The absent-`d` case is a tally before the page is dropped, which a plain `?` cannot carry.
    let d = envelope.get("d").and_then(|v| v.as_object());
    if d.is_none() {
        bump(
            verdict_totals
                .entry("envelope_missing_d".to_string())
                .or_insert(0),
            1,
        );
        return None;
    }
    let d = d?;

    let results = d.get("results").and_then(|v| v.as_str()).unwrap_or("");
    let count = d.get("count").and_then(|v| v.as_i64());
    let pager = d.get("pager").and_then(|v| v.as_str()).unwrap_or("");
    Some((results, count, pager))
}

/// Split one response's `results` string into rows.
fn rows_of<'a>(results: &'a str, tr: &regex::Regex) -> Vec<&'a str> {
    let row_parts: Vec<&str> = tr.split(results).collect();
    row_parts.into_iter().skip(1).collect()
}

/// Scan every raw body: its tallies, its per-page analysis and its samples.
pub(crate) fn scan(
    raw_files: &[(String, String)],
    join: &BTreeMap<String, Vec<EvidenceEntry>>,
    parsed_files_map: &BTreeMap<String, Value>,
    samples_limit: usize,
    verdict_totals: &mut IndexMap<String, usize>,
    regexes: &Regexes,
) -> Result<PageScan, Box<dyn std::error::Error>> {
    let mut out = PageScan {
        link_totals: rowcensus::link_total_slots(),
        ..PageScan::default()
    };
    let row_re = regexes.for_rows();

    for (digest, path) in raw_files {
        let blob = fs::read(path)?;
        record_digest_mismatch(digest, &blob, &mut out.digest_mismatch);

        let entries = join.get(digest).cloned().unwrap_or_default();
        record_byte_mismatch(digest, &entries, blob.len(), &mut out.byte_mismatch);

        let sport = sport_of(&entries);
        let slot = out
            .filter_totals
            .entry(
                sport
                    .clone()
                    .unwrap_or_else(|| "unknown/ambiguous".to_string()),
            )
            .or_insert(0);
        bump(slot, 1);

        let text = String::from_utf8_lossy(&blob);
        record_interstitials(&text, &mut out.interstitials);

        let Some(envelope) = envelope(&text, verdict_totals) else {
            continue;
        };
        let Some((results, count, pager)) = envelope_parts(&envelope, verdict_totals) else {
            continue;
        };

        let body = Body {
            digest,
            sport: &sport,
            rows: rows_of(results, &regexes.tr),
            count,
            pager,
            entries: &entries,
        };
        rowcensus::analyse_body(&mut out, &body, &row_re, parsed_files_map, samples_limit);
    }
    Ok(out)
}
