//! Counted inventory of live athletic.net /Search.aspx/runSearch response bodies (G1 slice).
//!
//! Read-only. Every number printed here is measured over the verbatim bytes the pipeline
//! retained, not over a re-fetch.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;

use clap::Parser;
use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};

// ── CLI ────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "g1-audit")]
struct Args {
    #[arg(long)]
    raw: Vec<String>,
    #[arg(long)]
    evidence: Vec<String>,
    #[arg(long)]
    parsed: Vec<String>,
    #[arg(long, default_value_t = 3)]
    samples: usize,
    #[arg(long, default_value = "set")]
    label: String,
}

// ── Constants ──────────────────────────────────────────────────────────

static ABS_PREFIXES: &[&str] = &[
    "https://www.athletic.net",
    "https://athletic.net",
    "http://www.athletic.net",
    "http://athletic.net",
];

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

// ── Data structures ────────────────────────────────────────────────────

#[derive(Clone)]
struct EvidenceEntry {
    query: Option<String>,
    sport: Option<String>,
    bytes: Option<String>,
    parsed: Option<String>,
}
struct EvidenceRecord {
    name: String,
    complete: bool,
    failure_messages: Vec<String>,
    pages_with_parsed: Vec<Option<String>>,
}
struct RowAnalysis {
    athlete_hrefs: Vec<String>,
    noncanonical: Vec<String>,
    sported: Vec<String>,
    issues: Vec<String>,
    candidate: bool,
}
struct PageAnalysis {
    digest: String,
    sport: Option<String>,
    queries: Vec<String>,
    count: Option<i64>,
    rows: usize,
    tf_hrefs: usize,
    xc_hrefs: usize,
    noncanonical: usize,
    row_issues: IndexMap<String, usize>,
    candidates: usize,
    has_next: bool,
    parser_candidates: Option<usize>,
    parser_issues: IndexMap<String, usize>,
    parser_count: Option<i64>,
    parser_verdicts: Vec<String>,
    parser_next: Option<i64>,
}

enum SampleEntry {
    Noncanonical(String, String),
    CountGtRows(String, Option<String>, i64, usize, String),
}

// ── Helpers ────────────────────────────────────────────────────────────

fn fmt_dict<K: AsRef<str>, V: std::fmt::Display>(items: &[(K, V)]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|(k, v)| format!("'{}': {}", k.as_ref(), v))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

fn json_bool_to_str(v: &Value) -> &'static str {
    match v {
        Value::Bool(true) => "True",
        Value::Bool(false) => "False",
        _ => "None",
    }
}

fn sha256_hex(blob: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(blob);
    format!("{:x}", hasher.finalize())
}

fn py_repr_str(s: &str) -> String {
    if s.contains('\'') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}

fn py_repr_value(v: &str, is_int: bool) -> String {
    if is_int {
        v.to_string()
    } else {
        py_repr_str(v)
    }
}

fn py_repr_tuple(elements: &[(&str, bool)]) -> String {
    let parts: Vec<String> = elements
        .iter()
        .map(|(s, is_int)| py_repr_value(s, *is_int))
        .collect();
    format!("({})", parts.join(", "))
}

/// Add `by` to a counter slot, saturating instead of wrapping.
///
/// Every counter here is a census of things that exist in memory at that moment (pages, rows, links
/// of the retained fixtures), so saturation is unreachable for a real corpus; the wrap-around it
/// replaces is not a value any caller of this report can use.
fn bump(slot: &mut usize, by: usize) {
    *slot = slot.saturating_add(by);
}

/// The envelope's `count` — an `i64` as the site reports it — read in the `usize` domain the page's
/// rows are measured in.
///
/// A negative `i64` is not a row count; it saturates to `usize::MAX` so it still compares as larger
/// than any page, which is the branch outcome the pre-repair cast produced for every negative value.
fn count_len(count: i64) -> usize {
    usize::try_from(count).unwrap_or(usize::MAX)
}

/// A `usize` length — a row or candidate count — read in the envelope's `i64` domain.
///
/// A length above `i64::MAX` would need that many live elements; saturating to `i64::MAX` keeps the
/// comparison against an envelope count total rather than wrapping into a negative.
fn len_count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// The leading 12 bytes of a digest, for the display lines that shorten one.
///
/// Digests are `sha256_hex` output: 64 ASCII bytes, so the cut is always on a boundary. A shorter or
/// non-boundary string is returned whole instead of panicking.
fn digest_head(digest: &str) -> &str {
    digest.split_at_checked(12).map_or(digest, |(head, _)| head)
}

// ── Core IO ────────────────────────────────────────────────────────────

fn read_dir(pattern_dirs: &[String]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for directory in pattern_dirs {
        let dir_path = PathBuf::from(directory);
        if !dir_path.is_dir() {
            continue;
        }
        let mut entries: Vec<_> = match fs::read_dir(&dir_path) {
            Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
            Err(_) => Vec::new(),
        };
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            if path.extension().map(|e| e == "body").unwrap_or(false) {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                out.push((stem.to_string(), path.to_str().unwrap_or("").to_string()));
            }
        }
    }
    out
}

// ── Regex analysis ─────────────────────────────────────────────────────

fn path_of(href: &str) -> &str {
    let mut value = href;
    for prefix in ABS_PREFIXES {
        if value.to_lowercase().starts_with(prefix) {
            // `to_lowercase` is not length-preserving for every code point, so `prefix.len()` is only
            // a candidate boundary: a URL that does not cut there is left whole instead of panicking.
            if let Some(tail) = value.get(prefix.len()..) {
                value = tail;
            }
            break;
        }
    }
    let value = value.split('?').next().unwrap_or(value);
    value.split('#').next().unwrap_or(value)
}

fn link_class(
    href: &str,
    athlete_re: &Regex,
    sported_re: &Regex,
    sportless_re: &Regex,
    empty_id_re: &Regex,
) -> (&'static str, Option<&'static str>) {
    if !athlete_re.is_match(href) {
        return ("not_athlete", None);
    }
    let path = path_of(href);
    if let Some(caps) = sported_re.captures(path) {
        let sport = caps
            .get(2)
            .map(|g| g.as_str().to_lowercase())
            .unwrap_or_default();
        let token = if sport == "track-and-field" {
            "tf"
        } else {
            "xc"
        };
        return ("sported", Some(token));
    }
    if sportless_re.is_match(path) {
        return ("sportless", None);
    }
    if empty_id_re.is_match(path) {
        return ("empty_id", None);
    }
    ("other_noncanonical", None)
}

/// The compiled patterns every row is tested against, bundled so `analyse_row` takes one argument
/// for them instead of six.
struct RowRegexes<'a> {
    href: &'a Regex,
    athlete: &'a Regex,
    sported: &'a Regex,
    sportless: &'a Regex,
    empty_id: &'a Regex,
    name: &'a Regex,
}

fn analyse_row(row_html: &str, sport: Option<&str>, re: &RowRegexes<'_>) -> RowAnalysis {
    let href_re = re.href;
    let athlete_re = re.athlete;
    let sported_re = re.sported;
    let sportless_re = re.sportless;
    let empty_id_re = re.empty_id;
    let name_re = re.name;
    let hrefs: Vec<String> = href_re
        .captures_iter(row_html)
        .filter_map(|m| m.get(1))
        .map(|g| g.as_str().to_string())
        .collect();

    let athlete: Vec<(String, &'static str, Option<&'static str>)> = hrefs
        .iter()
        .filter(|h| athlete_re.is_match(h))
        .map(|h| {
            let (cls, st) = link_class(h, athlete_re, sported_re, sportless_re, empty_id_re);
            (h.clone(), cls, st)
        })
        .collect();

    let noncanonical: Vec<String> = athlete
        .iter()
        .filter(|(_, cls, _)| *cls == "empty_id" || *cls == "other_noncanonical")
        .map(|(h, _, _)| h.clone())
        .collect();

    let sported: Vec<String> = athlete
        .iter()
        .filter(|(_, cls, _)| *cls == "sported")
        .filter_map(|(_, _, st)| st.map(|s| s.to_string()))
        .collect();

    let identity = athlete
        .iter()
        .any(|(_, cls, _)| *cls == "sported" || *cls == "sportless");

    let selected = match sport {
        Some(s) => sported.iter().any(|t| t == s),
        None => false,
    };

    let mut issues: Vec<String> = Vec::new();
    if !noncanonical.is_empty() {
        issues.push("athlete URL is not canonical".to_string());
    }
    if identity && !selected {
        issues.push("result row belongs to another or unspecified sport".to_string());
    }

    let name_matches: Vec<String> = name_re
        .captures_iter(row_html)
        .filter_map(|m| m.get(1))
        .map(|g| g.as_str().trim().to_string())
        .collect();

    if identity && selected && name_matches.first().is_some_and(|name| name.is_empty()) {
        issues.push("athlete display name is absent or exceeds bound".to_string());
    }

    let candidate = identity && selected && issues.is_empty();

    RowAnalysis {
        athlete_hrefs: athlete.iter().map(|(h, _, _)| h.clone()).collect(),
        noncanonical,
        sported,
        issues,
        candidate,
    }
}

// ── Main ───────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let raw_files = read_dir(&args.raw);
    let evidence_files = read_dir(&args.evidence);
    let parsed_files = read_dir(&args.parsed);

    println!(
        "== {} ==\nraw bodies: {}  evidence records: {}  parsed records: {}",
        args.label,
        raw_files.len(),
        evidence_files.len(),
        parsed_files.len(),
    );

    // ── Compile regexes ────────────────────────────────────────────────

    let tr_re = Regex::new(r"<tr[ >]")?;
    let href_re = Regex::new(r#"href=["']([^"']*)["']"#)?;
    let athlete_re = Regex::new(r"/athlete/")?;
    let sported_re = Regex::new(r"^/athlete/(\d+)/(track-and-field|cross-country)(/all)?/?$")?;
    let sportless_re = Regex::new(r"^/athlete/(\d+)/?$")?;
    let empty_id_re = Regex::new(r"^/athlete//")?;
    let name_re = Regex::new(
        r#"href=["']/athlete/\d+/(?:track-and-field|cross-country)(?:/all)?/?["'][^>]*>([^<]*)"#,
    )?;

    // ── Evidence processing ────────────────────────────────────────────

    let mut join: BTreeMap<String, Vec<EvidenceEntry>> = BTreeMap::new();
    let mut record_complete: IndexMap<String, usize> = IndexMap::new();
    let mut failures: IndexMap<String, usize> = IndexMap::new();
    let mut evidence_records: Vec<EvidenceRecord> = Vec::new();
    for (name, path) in &evidence_files {
        let blob = fs::read(path)?;
        let rec: Value = match serde_json::from_slice(&blob) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(cv) = rec.get("complete") {
            bump(
                record_complete
                    .entry(json_bool_to_str(cv).to_string())
                    .or_insert(0),
                1,
            );
        }

        if let Some(failures_arr) = rec.get("failures") {
            if let Some(arr) = failures_arr.as_array() {
                for failure in arr {
                    if let Some(obj) = failure.as_object() {
                        let code = obj.get("code").and_then(|v| v.as_str()).unwrap_or("");
                        let message = obj.get("message").and_then(|v| v.as_str()).unwrap_or("");
                        let empty_arr = Vec::<Value>::new();
                        let evidence_arr = obj
                            .get("evidence")
                            .and_then(|v| v.as_array())
                            .unwrap_or(&empty_arr);
                        let status: Vec<String> = evidence_arr
                            .iter()
                            .filter_map(|e| e.get("http_status").and_then(|v| v.as_i64()))
                            .map(|v| v.to_string())
                            .collect();
                        let status_str = if status.is_empty() {
                            "-".to_string()
                        } else {
                            status.join(",")
                        };
                        let key = format!("{}: {}  [http={}]", code, message, status_str);
                        bump(failures.entry(key).or_insert(0), 1);
                    } else {
                        bump(failures.entry(format!("{}", failure)).or_insert(0), 1);
                    }
                }
            }
        }

        if let Some(pages) = rec.get("pages") {
            if let Some(pages_arr) = pages.as_array() {
                let query_val = rec.get("query");
                let q_query = query_val
                    .and_then(|q| q.get("query"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let q_sport = query_val
                    .and_then(|q| q.get("sport"))
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "track_field" => "tf".to_string(),
                        "cross_country" => "xc".to_string(),
                        _ => s.to_string(),
                    });
                for page in pages_arr.iter() {
                    if let Some(response) = page.get("response") {
                        let digest = response
                            .get("digest")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        join.entry(digest).or_default().push(EvidenceEntry {
                            query: q_query.clone(),
                            sport: q_sport.clone(),
                            bytes: response
                                .get("bytes")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            parsed: page
                                .get("parsed")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        });
                    }
                }
            }
        }
        let complete = rec
            .get("complete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let failure_messages: Vec<String> = rec
            .get("failures")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|f| {
                        if let Some(obj) = f.as_object() {
                            obj.get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string()
                        } else {
                            f.to_string()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let pages_with_parsed: Vec<Option<String>> = rec
            .get("pages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|p| {
                        p.get("parsed")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        evidence_records.push(EvidenceRecord {
            name: name.clone(),
            complete,
            failure_messages,
            pages_with_parsed,
        });
    }

    // Print evidence summary
    let complete_items: Vec<_> = record_complete
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    println!("evidence: complete={}", fmt_dict(&complete_items));
    let total_failures: usize = failures.values().sum();
    println!("evidence failures ({} total):", total_failures);

    let mut mc: Vec<_> = failures.iter().map(|(k, v)| (k.clone(), *v)).collect();
    mc.sort_by(|(_, c1), (_, c2)| c2.cmp(c1));
    for (msg, count) in &mc {
        println!("  {:5}  {}", count, msg);
    }
    if failures.is_empty() {
        println!("  (none)");
    }

    // ── Parsed files index ─────────────────────────────────────────────

    let parsed_files_map: BTreeMap<String, Value> = parsed_files
        .iter()
        .map(|(name, path)| -> Result<_, Box<dyn std::error::Error>> {
            let blob = fs::read(path)?;
            let rec: Value = serde_json::from_slice(&blob)?;
            Ok::<_, Box<dyn std::error::Error>>((name.clone(), rec))
        })
        .collect::<Result<_, _>>()?;
    let parsed_records = parsed_files_map.clone();
    // ── Raw body analysis ──────────────────────────────────────────────

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

    let mut verdict_totals: IndexMap<String, usize> = IndexMap::new();
    let mut class_samples: IndexMap<String, Vec<SampleEntry>> = IndexMap::new();
    let mut count_vs_rows: IndexMap<String, usize> = IndexMap::new();
    let mut interstitials: IndexMap<String, usize> = IndexMap::new();
    let mut per_page: Vec<PageAnalysis> = Vec::new();
    let mut filter_totals: IndexMap<String, usize> = IndexMap::new();
    let mut digest_mismatch: Vec<(String, String)> = Vec::new();
    let mut byte_mismatch: Vec<(String, String, usize)> = Vec::new();
    let _records_crosscheck: IndexMap<String, usize> = IndexMap::new();
    let _failure_classes: IndexMap<String, usize> = IndexMap::new();
    let mut unexplained: Vec<(String, bool, bool, bool, Vec<String>)> = Vec::new();

    for (digest, path) in &raw_files {
        let blob = fs::read(path)?;
        let sha = sha256_hex(&blob);
        if sha != *digest {
            digest_mismatch.push((digest.clone(), sha));
        }

        let entries = join.get(digest).cloned().unwrap_or_default();
        for entry in &entries {
            if let Some(claimed) = &entry.bytes {
                if let Ok(claimed_bytes) = claimed.parse::<usize>() {
                    if claimed_bytes != blob.len() {
                        byte_mismatch.push((digest.clone(), claimed.clone(), blob.len()));
                    }
                }
            }
        }

        // Determine sport
        let sports: HashSet<&str> = entries.iter().filter_map(|e| e.sport.as_deref()).collect();
        let sport: Option<String> = if sports.len() == 1 {
            sports.iter().next().map(|token| match *token {
                "track_field" => "tf".to_string(),
                "cross_country" => "xc".to_string(),
                s => s.to_string(),
            })
        } else {
            None
        };
        filter_totals
            .entry(
                sport
                    .clone()
                    .unwrap_or_else(|| "unknown/ambiguous".to_string()),
            )
            .and_modify(|c| bump(c, 1))
            .or_insert(1);

        // Interstitial markers
        let text = String::from_utf8_lossy(&blob);
        let lower = text.to_lowercase();
        for marker in INTERSTITIAL_MARKERS {
            if lower.contains(marker) {
                bump(interstitials.entry(marker.to_string()).or_insert(0), 1);
            }
        }

        // Parse envelope
        let envelope: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                bump(
                    verdict_totals
                        .entry("envelope_unparseable".to_string())
                        .or_insert(0),
                    1,
                );
                continue;
            }
        };

        let d = envelope.get("d").and_then(|v| v.as_object());
        if d.is_none() {
            bump(
                verdict_totals
                    .entry("envelope_missing_d".to_string())
                    .or_insert(0),
                1,
            );
            continue;
        }
        let Some(d) = d else { continue };

        let results = d.get("results").and_then(|v| v.as_str()).unwrap_or("");
        let count = d.get("count").and_then(|v| v.as_i64());
        let pager = d.get("pager").and_then(|v| v.as_str()).unwrap_or("");

        // Split results into rows
        let row_parts: Vec<&str> = tr_re.split(results).collect();
        let rows: Vec<&str> = row_parts.into_iter().skip(1).collect();

        let row_analyses: Vec<_> = rows
            .iter()
            .map(|r| {
                analyse_row(
                    r,
                    sport.as_deref(),
                    &RowRegexes {
                        href: &href_re,
                        athlete: &athlete_re,
                        sported: &sported_re,
                        sportless: &sportless_re,
                        empty_id: &empty_id_re,
                        name: &name_re,
                    },
                )
            })
            .collect();

        let candidates: Vec<_> = row_analyses.iter().filter(|a| a.candidate).collect();
        let noncanonical_total: usize = row_analyses.iter().map(|a| a.noncanonical.len()).sum();
        let tf_hrefs: usize = row_analyses
            .iter()
            .flat_map(|a| &a.sported)
            .filter(|s| *s == "tf")
            .count();
        let xc_hrefs: usize = row_analyses
            .iter()
            .flat_map(|a| &a.sported)
            .filter(|s| *s == "xc")
            .count();
        bump(
            link_totals.entry("rows_total".to_string()).or_default(),
            rows.len(),
        );
        bump(
            link_totals
                .entry("rows_with_athlete_href".to_string())
                .or_default(),
            row_analyses
                .iter()
                .filter(|a| !a.athlete_hrefs.is_empty())
                .count(),
        );
        bump(
            link_totals
                .entry("rows_without_athlete_href".to_string())
                .or_default(),
            row_analyses
                .iter()
                .filter(|a| a.athlete_hrefs.is_empty())
                .count(),
        );
        bump(
            link_totals
                .entry("athlete_hrefs_total".to_string())
                .or_default(),
            row_analyses
                .iter()
                .map(|a| a.athlete_hrefs.len())
                .sum::<usize>(),
        );
        bump(
            link_totals
                .entry("athlete_hrefs_sported_tf".to_string())
                .or_default(),
            tf_hrefs,
        );
        bump(
            link_totals
                .entry("athlete_hrefs_sported_xc".to_string())
                .or_default(),
            xc_hrefs,
        );
        bump(
            link_totals
                .entry("athlete_hrefs_noncanonical".to_string())
                .or_default(),
            noncanonical_total,
        );
        bump(
            link_totals
                .entry("rows_multi_athlete_href".to_string())
                .or_default(),
            row_analyses
                .iter()
                .filter(|a| a.athlete_hrefs.len() > 1)
                .count(),
        );
        bump(
            link_totals
                .entry("rows_candidate_predicted".to_string())
                .or_default(),
            candidates.len(),
        );

        // Count vs rows
        if count.is_none() {
            bump(
                count_vs_rows.entry("count_absent".to_string()).or_insert(0),
                1,
            );
        } else if let Some(c) = count {
            match count_len(c).cmp(&rows.len()) {
                std::cmp::Ordering::Equal => {
                    bump(
                        count_vs_rows
                            .entry("count_eq_rows".to_string())
                            .or_insert(0),
                        1,
                    );
                }
                std::cmp::Ordering::Greater => {
                    bump(
                        count_vs_rows
                            .entry("count_gt_rows".to_string())
                            .or_insert(0),
                        1,
                    );
                }
                std::cmp::Ordering::Less => {
                    bump(
                        count_vs_rows
                            .entry("count_lt_rows".to_string())
                            .or_insert(0),
                        1,
                    );
                }
            }
        }

        // Pager
        let has_next = pager.contains("data-start");
        let _pager_li_count = pager.matches("<li").count();

        // Row issues (flat)
        let row_issues_flat: Vec<_> = row_analyses
            .iter()
            .flat_map(|a| a.issues.iter().cloned())
            .collect();
        let mut row_issues_map: IndexMap<String, usize> = IndexMap::new();
        for issue in &row_issues_flat {
            bump(row_issues_map.entry(issue.clone()).or_insert(0), 1);
        }

        // Parsed digests join
        let mut parsed_digests: Vec<_> = entries
            .iter()
            .filter_map(|e| e.parsed.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        parsed_digests.sort();
        let mut parser_issues: IndexMap<String, usize> = IndexMap::new();
        let mut parser_candidates_count: Option<usize> = None;
        let mut parser_count: Option<i64> = None;
        let mut parser_next: Option<i64> = None;

        for pd in &parsed_digests {
            if let Some(parser) = parsed_files_map.get(pd) {
                parser_count = Some(parser.get("count").and_then(|v| v.as_i64()).unwrap_or(0));
                parser_next = parser.get("next_offset").and_then(|v| v.as_i64());

                if let Some(arr) = parser.get("issues").and_then(|v| v.as_array()) {
                    for issue in arr {
                        if let Some(msg) = issue.get("message").and_then(|v| v.as_str()) {
                            bump(parser_issues.entry(msg.to_string()).or_insert(0), 1);
                        }
                    }
                }
                parser_candidates_count = parser
                    .get("candidates")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len());
            }
        }
        // Parser verdicts (per the Python logic)
        let mut parser_verdicts: Vec<String> = Vec::new();
        for pd in &parsed_digests {
            if let Some(parser) = parsed_files_map.get(pd) {
                let issues_here: Vec<String> = parser
                    .get("issues")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flat_map(|arr| {
                        arr.iter().filter_map(|i| {
                            i.get("message")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                    })
                    .collect();
                if !issues_here.is_empty() {
                    parser_verdicts.push("row_issues".to_string());
                } else {
                    let pc = parser.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                    let pn = parser.get("next_offset").and_then(|v| v.as_i64());
                    let cand = parser
                        .get("candidates")
                        .and_then(|v| v.as_array())
                        .map_or(0, |a| a.len());
                    let cand = len_count(cand);
                    if pc > cand && pn.is_none() {
                        parser_verdicts.push("short_page".to_string());
                    } else if pc == cand && pn.is_none() {
                        parser_verdicts.push("clean_full".to_string());
                    } else {
                        parser_verdicts.push("paginated".to_string());
                    }
                }
            }
        }

        // Per-page analysis
        let queries: Vec<String> = entries
            .iter()
            .filter_map(|e| e.query.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let _queries_str = queries.join(",");

        let row_issue_count: usize = row_issues_map.values().sum();
        let _row_issue_empty = row_issue_count == 0;
        per_page.push(PageAnalysis {
            digest: digest.clone(),
            sport: sport.clone(),
            queries,
            count,
            rows: rows.len(),
            tf_hrefs,
            xc_hrefs,
            noncanonical: noncanonical_total,
            row_issues: row_issues_map,
            candidates: candidates.len(),
            has_next,
            parser_candidates: parser_candidates_count,
            parser_issues,
            parser_count,
            parser_next,
            parser_verdicts,
        });

        // Samples
        for row_a in &row_analyses {
            for href in &row_a.noncanonical {
                let v = class_samples.entry("noncanonical".to_string()).or_default();
                if v.len() < args.samples {
                    v.push(SampleEntry::Noncanonical(digest.clone(), href.clone()));
                }
            }
        }

        if let Some(c) = count {
            if count_len(c) > rows.len() {
                let samples = class_samples
                    .entry("count_gt_rows".to_string())
                    .or_default();
                if samples.len() < args.samples {
                    let pager_text = pager.to_string();
                    let pager_head = if pager_text.len() > 200 {
                        match pager_text.split_at_checked(200) {
                            Some((head, _)) => head.to_string(),
                            None => pager_text.clone(),
                        }
                    } else {
                        pager_text
                    };
                    samples.push(SampleEntry::CountGtRows(
                        digest.clone(),
                        sport.clone(),
                        c,
                        rows.len(),
                        pager_head,
                    ));
                }
            }
        }
    }
    // ── Record outcomes: iterate over evidence records, NOT join entries ─

    let mut records_crosscheck: IndexMap<String, usize> = IndexMap::new();
    let mut failure_classes: IndexMap<String, usize> = IndexMap::new();
    for rec in &evidence_records {
        let failed = !rec.complete;
        let has_issue = rec.pages_with_parsed.iter().any(|parsed| {
            let is_issue = parsed_records
                .get(parsed.as_deref().unwrap_or(""))
                .map(|p| {
                    p.get("issues")
                        .and_then(|v| v.as_array())
                        .map(|a| !a.is_empty())
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            is_issue
        });
        let row_issue_failure = rec
            .failure_messages
            .iter()
            .any(|m| m.contains("unrelated result rows"));
        let short_failure = rec
            .failure_messages
            .iter()
            .any(|m| m.contains("ended before the advertised"));

        if !failed {
            bump(
                records_crosscheck
                    .entry("complete".to_string())
                    .or_insert(0),
                1,
            );
        } else {
            bump(
                records_crosscheck.entry("failed".to_string()).or_insert(0),
                1,
            );
        }
        bump(
            records_crosscheck
                .entry(format!(
                    "  {}:own_parse_has_row_issues={}",
                    if !failed { "complete" } else { "failed" },
                    if has_issue { "True" } else { "False" }
                ))
                .or_insert(0),
            1,
        );

        if failed {
            bump(
                records_crosscheck
                    .entry(format!(
                        "  failed:row_issue_failure={}",
                        if row_issue_failure { "True" } else { "False" }
                    ))
                    .or_insert(0),
                1,
            );
            bump(
                records_crosscheck
                    .entry(format!(
                        "  failed:short_failure={}",
                        if short_failure { "True" } else { "False" }
                    ))
                    .or_insert(0),
                1,
            );

            let exceed = rec
                .failure_messages
                .iter()
                .any(|m| m.contains("advertised result count exceeds"));
            let mut classes: Vec<&str> = Vec::new();
            if row_issue_failure {
                classes.push("row_issues");
            }
            if short_failure {
                classes.push("short_page");
            }
            if exceed {
                classes.push("advertised_count_exceeds");
            }
            bump(failure_classes.entry(classes.join("+")).or_insert(0), 1);
        }
        if !failed && has_issue {
            unexplained.push((
                rec.name.clone(),
                failed,
                has_issue,
                false,
                rec.failure_messages.clone(),
            ));
        }
        if failed && !has_issue && !row_issue_failure {
            bump(
                records_crosscheck
                    .entry("  FAILED_WITHOUT_ROW_ISSUES".to_string())
                    .or_insert(0),
                1,
            );
            unexplained.push((
                rec.name.clone(),
                failed,
                has_issue,
                short_failure,
                rec.failure_messages.clone(),
            ));
        }
    }

    // ── Page verdicts ──────────────────────────────────────────────────

    // ── Link inventory ─────────────────────────────────────────────────

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

    // ── Count vs rows ──────────────────────────────────────────────────

    let cvr_items: Vec<_> = count_vs_rows.iter().map(|(k, v)| (k.clone(), *v)).collect();
    println!("count vs rows: {}", fmt_dict(&cvr_items));
    println!("page verdicts:");

    // recount section
    let pages_carrying_candidate_rows: usize = per_page.iter().filter(|p| p.candidates > 0).count();
    let row_issue_pages: usize = per_page
        .iter()
        .filter(|p| p.row_issues.values().sum::<usize>() > 0)
        .count();
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
        .filter(|p| {
            p.count.map(|c| count_len(c) == p.rows).unwrap_or(false)
                && p.row_issues.values().sum::<usize>() == 0
        })
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
        .filter(|p| p.row_issues.values().sum::<usize>() > 0)
        .map(|p| p.candidates)
        .sum();
    println!(
        "    pages_carrying_candidate_rows={} of_which_row_issue_pages={} candidate_rows_lost_on_those_pages={}",
        pages_carrying_candidate_rows,
        per_page.iter().filter(|p| p.candidates > 0 && p.row_issues.values().sum::<usize>() > 0).count(),
        candidate_rows_lost
    );
    // ── Parser verdicts ────────────────────────────────────────────────

    let parser_issue_pages: usize = per_page
        .iter()
        .filter(|p| !p.parser_issues.is_empty())
        .count();
    let parser_short_pages: usize = per_page
        .iter()
        .filter(|p| {
            let mut v = p.parser_verdicts.clone();
            v.sort();
            v.dedup();
            v.contains(&"short_page".to_string())
        })
        .count();
    let parser_clean_pages: usize = per_page
        .iter()
        .filter(|p| {
            let mut v = p.parser_verdicts.clone();
            v.sort();
            v.dedup();
            v == vec!["clean_full".to_string()]
        })
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
        .filter(|p| {
            let mut v = p.parser_verdicts.clone();
            v.sort();
            v.dedup();
            v.is_empty()
        })
        .count();
    println!("  parser (its own records for these same bytes):");
    println!(
        "    pages={} row_issue_pages={} short_page={} clean_full={} no_parsed_record={}",
        per_page.len(),
        parser_issue_pages,
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
    // ── Populate verdict_totals ────────────────────────────────────────

    let _parser_paginated: usize = per_page
        .iter()
        .filter(|p| {
            let mut v = p.parser_verdicts.clone();
            v.sort();
            v.dedup();
            v == vec!["paginated".to_string()]
        })
        .count();
    let parser_clean_pages_set: usize = per_page
        .iter()
        .filter(|p| p.parser_verdicts.contains(&"clean_full".to_string()))
        .count();
    let parser_paginated_set: usize = per_page
        .iter()
        .filter(|p| p.parser_verdicts.contains(&"paginated".to_string()))
        .count();
    let parser_short_pages_set: usize = per_page
        .iter()
        .filter(|p| p.parser_verdicts.contains(&"short_page".to_string()))
        .count();
    if parser_clean_pages_set > 0 {
        bump(
            verdict_totals
                .entry("parser_page:clean_full".to_string())
                .or_insert(0),
            parser_clean_pages_set,
        );
    }
    if parser_paginated_set > 0 {
        bump(
            verdict_totals
                .entry("parser_page:paginated".to_string())
                .or_insert(0),
            parser_paginated_set,
        );
    }
    if parser_short_pages_set > 0 {
        bump(
            verdict_totals
                .entry("parser_page:short_page".to_string())
                .or_insert(0),
            parser_short_pages_set,
        );
    }
    if parser_issue_pages > 0 {
        bump(
            verdict_totals
                .entry("parser_page:row_issues".to_string())
                .or_insert(0),
            parser_issue_pages,
        );
    }
    for page in &per_page {
        for (issue_msg, count) in &page.row_issues {
            bump(
                verdict_totals
                    .entry(format!("row_issue:{}", issue_msg))
                    .or_insert(0),
                *count,
            );
        }
        // ambiguous_parser_verdict: pages where parser_verdicts has multiple distinct verdicts
        let mut v = page.parser_verdicts.clone();
        v.sort();
        v.dedup();
        if v.len() > 1 {
            bump(
                verdict_totals
                    .entry("ambiguous_parser_verdict".to_string())
                    .or_insert(0),
                1,
            );
        }
    }

    // ── Issue classes on row-issue pages ─────────────────────────────────
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

    // ── Co-mingled sports ────────────────────────────────────────────────

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

    // ── Pagination analysis ─────────────────────────────────────────────

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

    // ── Record outcomes ────────────────────────────────────────────────

    println!("  record outcomes (the pipeline's own complete/failures) vs page verdicts:");
    let mut rc_items: Vec<_> = records_crosscheck
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    rc_items.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    for (key, value) in &rc_items {
        println!("    {:5}  {}", value, key);
    }

    // ── Rejected-record classes ────────────────────────────────────────

    println!("  rejected-record classes (which guard killed the query):");
    let mut fc_items: Vec<_> = failure_classes
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    fc_items.sort_by(|(_, c1), (_, c2)| c2.cmp(c1));
    for (key, value) in &fc_items {
        println!("    {:5}  {}", value, key);
    }
    // ── Unexplained ────────────────────────────────────────────────────

    let bool_py = |b: &bool| -> &'static str {
        if *b {
            "True"
        } else {
            "False"
        }
    };
    for (name, failed, has_issue, has_short, msgs) in unexplained.iter().take(4) {
        let short_name = name
            .split_at_checked(12)
            .map_or(name.as_str(), |(head, _)| head);
        let msg_str = match (msgs.first(), msgs.get(1)) {
            (Some(first), Some(second)) => format!("['{}', '{}']", first, second),
            (Some(first), None) => format!("['{}']", first),
            (None, _) => "[]".to_string(),
        };
        println!(
            "    unexplained {} failed={} page_issues={} page_short={} {}",
            short_name,
            bool_py(failed),
            bool_py(has_issue),
            bool_py(has_short),
            msg_str
        );
    }
    // ── Print verdicts ─────────────────────────────────────────────────

    println!("verdict totals:");
    let mut vt_items: Vec<_> = verdict_totals
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    vt_items.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    for (key, value) in &vt_items {
        println!("  {:7}  {}", value, key);
    }

    // ── Samples ────────────────────────────────────────────────────────

    // ── Page filter ────────────────────────────────────────────────────

    let pf_items: Vec<_> = filter_totals.iter().map(|(k, v)| (k.clone(), *v)).collect();
    println!(
        "page filter (sport the page was fetched for): {}",
        fmt_dict(&pf_items)
    );

    // ── Interstitial markers ───────────────────────────────────────────

    if interstitials.is_empty() {
        println!("interstitial markers: none");
    } else {
        let im_items: Vec<_> = interstitials.iter().map(|(k, v)| (k.clone(), *v)).collect();
        println!("interstitial markers: {}", fmt_dict(&im_items));
    }

    // ── SHA256 / byte mismatches ───────────────────────────────────────

    println!(
        "raw-body sha256 mismatches: {}  evidence-bytes mismatches: {}",
        digest_mismatch.len(),
        byte_mismatch.len()
    );
    for (digest, sha) in &digest_mismatch {
        println!(
            "  sha mismatch {} -> {}",
            digest_head(digest),
            digest_head(sha)
        );
    }
    for (digest, claimed, actual) in &byte_mismatch {
        println!(
            "  bytes mismatch {} claimed={} actual={}",
            digest_head(digest),
            claimed,
            actual
        );
    }
    println!("samples:");
    for (name, values) in &class_samples {
        for value in values {
            match value {
                SampleEntry::Noncanonical(digest, href) => {
                    let d = digest_head(digest);
                    let elements: Vec<_> = vec![(d, false), (href.as_str(), false)];
                    println!("  [{}] {}", name, py_repr_tuple(&elements));
                }
                SampleEntry::CountGtRows(digest, sport, count, rows, pager) => {
                    let d = digest_head(digest);
                    let sport_str = sport.as_deref().unwrap_or("None");
                    let e1 = d;
                    let e2 = sport_str;
                    let e3 = count.to_string();
                    let e4 = rows.to_string();
                    let e5 = pager.as_str();
                    let elements: Vec<_> = vec![
                        (e1, false),
                        (e2, sport.is_none()),
                        (e3.as_str(), true),
                        (e4.as_str(), true),
                        (e5, false),
                    ];
                    println!("  [{}] {}", name, py_repr_tuple(&elements));
                }
            }
        }
    }

    // ── Per-page ───────────────────────────────────────────────────────

    println!(
        "per-page (digest, filter, count, rows, tf, xc, noncanon, predicted issues, candidates, parser candidates/issues):"
    );
    for page in &per_page {
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
        let mut queries_sorted: Vec<&str> = page.queries.iter().map(|s| &**s).collect();
        queries_sorted.sort();
        let queries_str = queries_sorted.join(",");
        let queries_trunc = queries_str
            .split_at_checked(60)
            .map_or(queries_str.as_str(), |(head, _)| head)
            .to_string();
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

    Ok(())
}
