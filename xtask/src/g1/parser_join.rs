use std::collections::{BTreeMap, HashSet};

use indexmap::IndexMap;
use serde_json::Value;

use crate::counts::{bump, len_count};
use crate::evidence::EvidenceEntry;

pub(crate) struct ParserView {
    pub(crate) candidates: Option<usize>,
    pub(crate) issues: IndexMap<String, usize>,
    pub(crate) count: Option<i64>,
    pub(crate) next: Option<i64>,
    pub(crate) verdicts: Vec<String>,
}

fn parsed_digests(entries: &[EvidenceEntry]) -> Vec<String> {
    let mut digests: Vec<_> = entries
        .iter()
        .filter_map(|e| e.parsed.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    digests.sort();
    digests
}

fn totals(digests: &[String], parsed_files_map: &BTreeMap<String, Value>, view: &mut ParserView) {
    for pd in digests {
        if let Some(parser) = parsed_files_map.get(pd) {
            view.count = Some(
                parser
                    .get("count")
                    .and_then(|v| v.as_i64())
                    .map_or(0, core::convert::identity),
            );
            view.next = parser.get("next_offset").and_then(|v| v.as_i64());

            if let Some(arr) = parser.get("issues").and_then(|v| v.as_array()) {
                for issue in arr {
                    if let Some(msg) = issue.get("message").and_then(|v| v.as_str()) {
                        bump(view.issues.entry(msg.to_string()).or_insert(0), 1);
                    }
                }
            }
            view.candidates = parser
                .get("candidates")
                .and_then(|v| v.as_array())
                .map(|a| a.len());
        }
    }
}

fn verdict_of(parser: &Value) -> String {
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
        return "row_issues".to_string();
    }
    let count = parser
        .get("count")
        .and_then(|v| v.as_i64())
        .map_or(0, core::convert::identity);
    let next = parser.get("next_offset").and_then(|v| v.as_i64());
    let candidates = parser
        .get("candidates")
        .and_then(|v| v.as_array())
        .map_or(0, |a| a.len());
    let candidates = len_count(candidates);
    if count > candidates && next.is_none() {
        "short_page".to_string()
    } else if count == candidates && next.is_none() {
        "clean_full".to_string()
    } else {
        "paginated".to_string()
    }
}

fn verdicts(digests: &[String], parsed_files_map: &BTreeMap<String, Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for pd in digests {
        if let Some(parser) = parsed_files_map.get(pd) {
            out.push(verdict_of(parser));
        }
    }
    out
}

pub(crate) fn view(
    entries: &[EvidenceEntry],
    parsed_files_map: &BTreeMap<String, Value>,
) -> ParserView {
    let digests = parsed_digests(entries);
    let mut view = ParserView {
        candidates: None,
        issues: IndexMap::new(),
        count: None,
        next: None,
        verdicts: Vec::new(),
    };
    totals(&digests, parsed_files_map, &mut view);
    view.verdicts = verdicts(&digests, parsed_files_map);
    view
}
