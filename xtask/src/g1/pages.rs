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

pub(crate) struct PageAnalysis {
    pub(crate) digest: String,
    pub(crate) sport: Option<String>,
    pub(crate) queries: Vec<String>,
    pub(crate) count: Option<i64>,
    pub(crate) rows: usize,
    pub(crate) tf_hrefs: usize,
    pub(crate) xc_hrefs: usize,
    pub(crate) noncanonical: usize,
    pub(crate) row_issues: IndexMap<String, usize>,
    pub(crate) candidates: usize,
    pub(crate) has_next: bool,
    pub(crate) parser_candidates: Option<usize>,
    pub(crate) parser_issues: IndexMap<String, usize>,
    pub(crate) parser_count: Option<i64>,
    pub(crate) parser_next: Option<i64>,
    pub(crate) parser_verdicts: Vec<String>,
}

#[derive(Default)]
pub(crate) struct PageScan {
    pub(crate) per_page: Vec<PageAnalysis>,
    pub(crate) link_totals: IndexMap<String, usize>,
    pub(crate) count_vs_rows: IndexMap<String, usize>,
    pub(crate) interstitials: IndexMap<String, usize>,
    pub(crate) filter_totals: IndexMap<String, usize>,
    pub(crate) digest_mismatch: Vec<(String, String)>,
    pub(crate) byte_mismatch: Vec<(String, String, usize)>,
    pub(crate) class_samples: Samples,
}

pub(crate) struct Body<'a> {
    pub(crate) digest: &'a str,
    pub(crate) sport: &'a Option<String>,
    pub(crate) rows: Vec<&'a str>,
    pub(crate) count: Option<i64>,
    pub(crate) pager: &'a str,
    pub(crate) entries: &'a [EvidenceEntry],
}

fn record_digest_mismatch(digest: &str, blob: &[u8], out: &mut Vec<(String, String)>) {
    let sha = sha256_hex(blob);
    if sha != *digest {
        out.push((digest.to_string(), sha));
    }
}

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

fn record_interstitials(text: &str, interstitials: &mut IndexMap<String, usize>) {
    let lower = text.to_lowercase();
    for marker in INTERSTITIAL_MARKERS {
        if lower.contains(marker) {
            bump(interstitials.entry(marker.to_string()).or_insert(0), 1);
        }
    }
}

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

fn envelope_parts<'a>(
    envelope: &'a Value,
    verdict_totals: &mut IndexMap<String, usize>,
) -> Option<(&'a str, Option<i64>, &'a str)> {
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

    let results = d
        .get("results")
        .and_then(|v| v.as_str())
        .map_or("", core::convert::identity);
    let count = d.get("count").and_then(|v| v.as_i64());
    let pager = d
        .get("pager")
        .and_then(|v| v.as_str())
        .map_or("", core::convert::identity);
    Some((results, count, pager))
}

fn rows_of<'a>(results: &'a str, tr: &regex::Regex) -> Vec<&'a str> {
    let row_parts: Vec<&str> = tr.split(results).collect();
    row_parts.into_iter().skip(1).collect()
}

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

        let entries = join
            .get(digest)
            .cloned()
            .map_or(Default::default(), core::convert::identity);
        record_byte_mismatch(digest, &entries, blob.len(), &mut out.byte_mismatch);

        let sport = sport_of(&entries);
        let slot = out
            .filter_totals
            .entry(match sport.clone() {
                Some(sport) => sport,
                None => "unknown/ambiguous".to_string(),
            })
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
