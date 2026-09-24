//! The evidence records: which retained bodies they claim, how each record ended, and the tallies the
//! report prints for both.

use std::collections::BTreeMap;
use std::fs;

use indexmap::IndexMap;
use serde_json::Value;

use crate::counts::bump;
use crate::pyrepr::{fmt_dict, json_bool_to_str};

/// One page of one evidence record: what query fetched it and which documents it points at.
#[derive(Clone)]
pub(crate) struct EvidenceEntry {
    /// The query the page was fetched for, when the record names one.
    pub(crate) query: Option<String>,
    /// The sport the page was fetched for, as the query recorded it.
    pub(crate) sport: Option<String>,
    /// The response body's byte count as the record states it, for the mismatch check.
    pub(crate) bytes: Option<String>,
    /// The digest of the parser's record over these bytes, when the page has one.
    pub(crate) parsed: Option<String>,
}

/// One evidence record, reduced to what the cross-check against page verdicts needs.
pub(crate) struct EvidenceRecord {
    /// The record's file stem.
    pub(crate) name: String,
    /// The record's own `complete` flag (absent reads as false).
    pub(crate) complete: bool,
    /// The `message` of each failure the record reported.
    pub(crate) failure_messages: Vec<String>,
    /// The parsed digest of each page, `None` when a page carries none.
    pub(crate) pages_with_parsed: Vec<Option<String>>,
}

/// Every evidence record's contribution: the body join, the tallies and the records themselves.
#[derive(Default)]
pub(crate) struct EvidenceIndex {
    /// Body digest -> the entries whose page response carries that digest.
    pub(crate) join: BTreeMap<String, Vec<EvidenceEntry>>,
    /// The records' `complete` flag as the report prints it, counted.
    pub(crate) record_complete: IndexMap<String, usize>,
    /// Failure messages as `code: message  [http=...]`, counted.
    pub(crate) failures: IndexMap<String, usize>,
    /// One entry per record, in directory order.
    pub(crate) records: Vec<EvidenceRecord>,
}

/// One failure as the summary line prints it: `code: message  [http=status,status]`.
fn failure_key(failure: &Value) -> String {
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
        format!("{}: {}  [http={}]", code, message, status_str)
    } else {
        format!("{}", failure)
    }
}

/// The `message` of each failure object, or the raw JSON of a failure that is not an object.
fn failure_messages(rec: &Value) -> Vec<String> {
    rec.get("failures")
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
        .unwrap_or_default()
}

/// The `parsed` digest of each page, `None` when a page carries none.
fn pages_with_parsed(rec: &Value) -> Vec<Option<String>> {
    rec.get("pages")
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
        .unwrap_or_default()
}

impl EvidenceIndex {
    /// Count the record's `complete` flag as the report prints it.
    fn absorb_complete(&mut self, rec: &Value) {
        if let Some(cv) = rec.get("complete") {
            bump(
                self.record_complete
                    .entry(json_bool_to_str(cv).to_string())
                    .or_insert(0),
                1,
            );
        }
    }

    /// Count every failure message the record reported.
    fn absorb_failures(&mut self, rec: &Value) {
        if let Some(failures_arr) = rec.get("failures") {
            if let Some(arr) = failures_arr.as_array() {
                for failure in arr {
                    bump(self.failures.entry(failure_key(failure)).or_insert(0), 1);
                }
            }
        }
    }

    /// Index every page of the record under the digest of the body it was fetched from.
    fn absorb_pages(&mut self, rec: &Value) {
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
                        self.join.entry(digest).or_default().push(EvidenceEntry {
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
    }

    /// Keep the record itself for the cross-check against page verdicts.
    fn absorb_record(&mut self, name: &str, rec: &Value) {
        let complete = rec
            .get("complete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.records.push(EvidenceRecord {
            name: name.to_string(),
            complete,
            failure_messages: failure_messages(rec),
            pages_with_parsed: pages_with_parsed(rec),
        });
    }

    /// Take everything one evidence record contributes.
    fn absorb(&mut self, name: &str, rec: &Value) {
        self.absorb_complete(rec);
        self.absorb_failures(rec);
        self.absorb_pages(rec);
        self.absorb_record(name, rec);
    }

    /// The evidence summary: the complete flag tally and the failure messages by count.
    pub(crate) fn print_summary(&self) {
        let complete_items: Vec<_> = self
            .record_complete
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        println!("evidence: complete={}", fmt_dict(&complete_items));
        let total_failures: usize = self.failures.values().sum();
        println!("evidence failures ({} total):", total_failures);

        let mut mc: Vec<_> = self.failures.iter().map(|(k, v)| (k.clone(), *v)).collect();
        mc.sort_by(|(_, c1), (_, c2)| c2.cmp(c1));
        for (msg, count) in &mc {
            println!("  {:5}  {}", count, msg);
        }
        if self.failures.is_empty() {
            println!("  (none)");
        }
    }
}

/// Read every evidence record: a record whose bytes are not JSON is skipped, as it always was.
pub(crate) fn index(
    evidence_files: &[(String, String)],
) -> Result<EvidenceIndex, Box<dyn std::error::Error>> {
    let mut index = EvidenceIndex::default();
    for (name, path) in evidence_files {
        let blob = fs::read(path)?;
        let rec: Value = match serde_json::from_slice(&blob) {
            Ok(v) => v,
            Err(_) => continue,
        };
        index.absorb(name, &rec);
    }
    Ok(index)
}
