//! The cross-check between the pipeline's own record verdicts and the page verdicts in the bytes.

use std::collections::BTreeMap;

use indexmap::IndexMap;
use serde_json::Value;

use crate::counts::tally;
use crate::evidence::EvidenceRecord;

/// One record the page verdicts do not explain: what it claimed and what the bytes showed.
struct Unexplained {
    /// The record's file stem.
    name: String,
    /// Whether the record reported itself incomplete.
    failed: bool,
    /// Whether a page it holds has a parsed record with issues.
    has_issue: bool,
    /// Whether one of its failures was a short page.
    has_short: bool,
    /// The record's failure messages.
    messages: Vec<String>,
}

/// The four flags one record's verdict reads off its own record.
struct RecordState {
    /// The record reported itself incomplete.
    failed: bool,
    /// A page it holds has a parsed record that reported issues.
    has_issue: bool,
    /// A failure message names unrelated result rows.
    row_issue_failure: bool,
    /// A failure message names a walk that ended early.
    short_failure: bool,
}

/// The record-level verdicts, and the records they leave unexplained.
#[derive(Default)]
pub(crate) struct Outcomes {
    /// `complete` / `failed` and the per-record combinations, counted.
    crosscheck: IndexMap<String, usize>,
    /// The guard that rejected each failed record, counted.
    failure_classes: IndexMap<String, usize>,
    /// The records no page verdict explains, in record order.
    unexplained: Vec<Unexplained>,
}

/// Whether any page of the record has a parsed record that reported issues.
fn own_parse_has_row_issues(
    rec: &EvidenceRecord,
    parsed_records: &BTreeMap<String, Value>,
) -> bool {
    rec.pages_with_parsed.iter().any(|parsed| {
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
    })
}

/// A Python boolean name, as the report prints it.
fn py_bool(b: bool) -> &'static str {
    if b {
        "True"
    } else {
        "False"
    }
}

impl RecordState {
    /// Read one record's four flags.
    fn of(rec: &EvidenceRecord, parsed_records: &BTreeMap<String, Value>) -> Self {
        let failed = !rec.complete;
        let has_issue = own_parse_has_row_issues(rec, parsed_records);
        let row_issue_failure = rec
            .failure_messages
            .iter()
            .any(|m| m.contains("unrelated result rows"));
        let short_failure = rec
            .failure_messages
            .iter()
            .any(|m| m.contains("ended before the advertised"));
        Self {
            failed,
            has_issue,
            row_issue_failure,
            short_failure,
        }
    }

    /// The cross-check key for what the record's own parser said about its pages.
    fn own_parse_key(&self) -> String {
        format!(
            "  {}:own_parse_has_row_issues={}",
            if self.failed { "failed" } else { "complete" },
            py_bool(self.has_issue)
        )
    }

    /// The unexplained entry this record contributes.
    fn unexplained(&self, rec: &EvidenceRecord, has_short: bool) -> Unexplained {
        Unexplained {
            name: rec.name.clone(),
            failed: self.failed,
            has_issue: self.has_issue,
            has_short,
            messages: rec.failure_messages.clone(),
        }
    }
}

/// A failed record: which failure shapes it carried, and the class they add up to.
fn record_failure(out: &mut Outcomes, rec: &EvidenceRecord, state: &RecordState) {
    let row_issue_key = format!(
        "  failed:row_issue_failure={}",
        py_bool(state.row_issue_failure)
    );
    tally(&mut out.crosscheck, &row_issue_key, 1);
    let short_key = format!("  failed:short_failure={}", py_bool(state.short_failure));
    tally(&mut out.crosscheck, &short_key, 1);

    let exceed = rec
        .failure_messages
        .iter()
        .any(|m| m.contains("advertised result count exceeds"));
    let mut classes: Vec<&str> = Vec::new();
    if state.row_issue_failure {
        classes.push("row_issues");
    }
    if state.short_failure {
        classes.push("short_page");
    }
    if exceed {
        classes.push("advertised_count_exceeds");
    }
    tally(&mut out.failure_classes, &classes.join("+"), 1);
}

/// Take one record's contribution to the cross-check.
fn absorb_record(
    out: &mut Outcomes,
    rec: &EvidenceRecord,
    parsed_records: &BTreeMap<String, Value>,
) {
    let state = RecordState::of(rec, parsed_records);
    tally(
        &mut out.crosscheck,
        if state.failed { "failed" } else { "complete" },
        1,
    );
    let own_parse_key = state.own_parse_key();
    tally(&mut out.crosscheck, &own_parse_key, 1);

    if state.failed {
        record_failure(out, rec, &state);
    }
    if !state.failed && state.has_issue {
        out.unexplained.push(state.unexplained(rec, false));
    }
    if state.failed && !state.has_issue && !state.row_issue_failure {
        tally(&mut out.crosscheck, "  FAILED_WITHOUT_ROW_ISSUES", 1);
        let has_short = state.short_failure;
        out.unexplained.push(state.unexplained(rec, has_short));
    }
}

/// Cross-check every evidence record against the page verdicts, in record order.
pub(crate) fn crosscheck(
    records: &[EvidenceRecord],
    parsed_records: &BTreeMap<String, Value>,
) -> Outcomes {
    let mut out = Outcomes::default();
    for rec in records {
        absorb_record(&mut out, rec, parsed_records);
    }
    out
}

/// The record outcomes, keyed as the report prints them.
pub(crate) fn print_crosscheck(out: &Outcomes) {
    println!("  record outcomes (the pipeline's own complete/failures) vs page verdicts:");
    let mut rc_items: Vec<_> = out
        .crosscheck
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    rc_items.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    for (key, value) in &rc_items {
        println!("    {:5}  {}", value, key);
    }
}

/// The rejected-record classes, most frequent first.
pub(crate) fn print_failure_classes(out: &Outcomes) {
    println!("  rejected-record classes (which guard killed the query):");
    let mut fc_items: Vec<_> = out
        .failure_classes
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    fc_items.sort_by(|(_, c1), (_, c2)| c2.cmp(c1));
    for (key, value) in &fc_items {
        println!("    {:5}  {}", value, key);
    }
}

/// The first four records no page verdict explains.
pub(crate) fn print_unexplained(out: &Outcomes) {
    for item in out.unexplained.iter().take(4) {
        let short_name = item
            .name
            .split_at_checked(12)
            .map_or(item.name.as_str(), |(head, _)| head);
        let msg_str = match (item.messages.first(), item.messages.get(1)) {
            (Some(first), Some(second)) => format!("['{}', '{}']", first, second),
            (Some(first), None) => format!("['{}']", first),
            (None, _) => "[]".to_string(),
        };
        println!(
            "    unexplained {} failed={} page_issues={} page_short={} {}",
            short_name,
            py_bool(item.failed),
            py_bool(item.has_issue),
            py_bool(item.has_short),
            msg_str
        );
    }
}
