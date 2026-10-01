use super::super::fetch::fetch_result_set;
use super::super::map::absorb_result_set;
use super::super::raw::RawPage;
use super::super::raw_issue::RawGradeIssueKind;
use super::super::wire::ResultSetRef;
use super::{Accumulator, Stats};
use crate::AdapterContext;
use census_domain::model::SchoolId;
use census_domain::school_index::SchoolIndex;
use std::collections::{HashMap, HashSet};

pub(super) struct Run {
    pub(super) index: SchoolIndex,
    pub(super) resolved: HashMap<String, Option<SchoolId>>,
    pub(super) stats: Stats,
    pub(super) accumulated: Accumulator,
    pub(super) done: HashSet<String>,
    pub(super) pending: Vec<(String, serde_json::Value)>,
}

impl Run {
    pub(super) async fn read(&mut self, ctx: &AdapterContext<'_>, reference: &ResultSetRef) {
        let key = format!("{}/{}", reference.meet_id, reference.rsid);
        if self.done.contains(&key) {
            self.stats.result_sets_resumed = self.stats.result_sets_resumed.saturating_add(1);
            return;
        }
        let page = match fetch_result_set(ctx.fetcher, reference, &ctx.fetch_options()).await {
            Ok(page) => page,
            Err(error) => {
                self.record_failure(reference, &key, &error);
                return;
            }
        };
        self.record_page(ctx, reference, key, page);
    }

    fn record_page(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
        key: String,
        page: RawPage,
    ) {
        self.stats.result_sets = self.stats.result_sets.saturating_add(1);
        self.stats.skipped_lines = self.stats.skipped_lines.saturating_add(page.skipped.len());
        if page.meet.rows_parsed == 0 && page.skipped.is_empty() {
            self.stats.result_sets_empty = self.stats.result_sets_empty.saturating_add(1);
        } else if page.meet.rows_parsed > 0 {
            absorb_result_set(
                &page,
                reference,
                &ctx.observed_on,
                &self.index,
                &mut self.resolved,
                &mut self.stats,
                &mut self.accumulated,
            );
        }
        let entry = journal_entry(&key, &page);
        let payload = journal_payload(reference, &entry, &page);
        if entry.complete {
            self.pending.push((entry.key, payload));
        } else {
            self.stats.failures.push(format!(
                "{}: partial raw parse; {} located row/field rejection(s), {} unrecognized grade token(s)",
                reference.url, entry.rejected, entry.unresolved_grades
            ));
            self.pending.push((entry.key, payload));
        }
    }

    fn record_failure(&mut self, reference: &ResultSetRef, key: &str, error: &crate::CrawlError) {
        let reason = format!("{}: {error}", reference.url);
        if matches!(error, crate::CrawlError::Schema { .. }) {
            self.pending.push((
                format!("partial/{key}"),
                serde_json::json!({
                    "meet": reference.meet_id,
                    "rsid": reference.rsid,
                    "source_url": reference.url,
                    "disposition": "rejected",
                    "document_rejection": reason,
                }),
            ));
        }
        self.stats.failures.push(reason);
    }

    pub(super) fn reject(&mut self, entry: &str) {
        self.stats
            .failures
            .push(format!("{entry}: not a /meets/<id>/results/<rsid>/raw URL"));
    }
}

struct JournalEntry {
    key: String,
    complete: bool,
    rejected: usize,
    unresolved_grades: usize,
}

fn journal_entry(key: &str, page: &RawPage) -> JournalEntry {
    let rejected = page.skipped.len();
    let unresolved_grades = page
        .grade_issues
        .iter()
        .filter(|issue| issue.kind == RawGradeIssueKind::Unrecognized)
        .count();
    match rejected == 0 && unresolved_grades == 0 {
        true => JournalEntry {
            key: key.to_string(),
            complete: true,
            rejected,
            unresolved_grades,
        },
        false => JournalEntry {
            key: format!("partial/{key}"),
            complete: false,
            rejected,
            unresolved_grades,
        },
    }
}

fn journal_payload(
    reference: &ResultSetRef,
    entry: &JournalEntry,
    page: &RawPage,
) -> serde_json::Value {
    serde_json::json!({
        "meet": reference.meet_id,
        "rsid": reference.rsid,
        "rows": page.meet.rows_parsed,
        "skipped_lines": entry.rejected,
        "rejections": &page.skipped,
        "grade_issues": &page.grade_issues,
        "source_url": reference.url,
        "disposition": if entry.complete { "complete" } else { "partial" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::milesplit::{RawGradeIssue, SourceRowLocator};
    use crate::result_file::ParsedMeet;
    use census_domain::model::SchoolYear;

    const REFERENCE_URL: &str =
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw";

    fn issue(token: &str, kind: RawGradeIssueKind) -> RawGradeIssue {
        RawGradeIssue {
            row: SourceRowLocator {
                ordinal: 25,
                byte_offset: 1_000,
                byte_length: 80,
            },
            raw_token: token.to_string(),
            kind,
        }
    }

    fn page(grade_issues: Vec<RawGradeIssue>) -> RawPage {
        RawPage {
            meet: ParsedMeet {
                name: "Beaver Eastern Invite".to_string(),
                date: "2026-09-19".to_string(),
                end_date: None,
                timer: None,
                events: Vec::new(),
                rows_parsed: 80,
                rows_skipped: 0,
            },
            sport: None,
            region: Some("OH".to_string()),
            school_year: SchoolYear::new(2026).expect("2026 is a season"),
            skipped: Vec::new(),
            grade_issues,
        }
    }

    #[test]
    fn located_grade_eight_issues_do_not_owe_a_partial_entry() {
        let page = page(vec![issue("8", RawGradeIssueKind::OutsideHighSchool)]);
        let entry = journal_entry("770621/1321880", &page);
        assert_eq!(entry.key, "770621/1321880");
        assert!(entry.complete);
        assert_eq!(entry.rejected, 0);
        assert_eq!(entry.unresolved_grades, 0);
        let reference = ResultSetRef::parse(REFERENCE_URL).expect("a raw result set URL");
        let payload = journal_payload(&reference, &entry, &page);
        assert_eq!(payload["disposition"], "complete");
        assert_eq!(payload["grade_issues"][0]["kind"], "outside_high_school");
    }

    #[test]
    fn a_malformed_grade_token_owes_a_partial_entry_at_its_own_key() {
        let page = page(vec![issue("XX", RawGradeIssueKind::Unrecognized)]);
        let entry = journal_entry("770621/1321880", &page);
        assert_eq!(entry.key, "partial/770621/1321880");
        assert!(!entry.complete);
        assert_eq!(entry.unresolved_grades, 1);
        let reference = ResultSetRef::parse(REFERENCE_URL).expect("a raw result set URL");
        let payload = journal_payload(&reference, &entry, &page);
        assert_eq!(payload["source_url"], reference.url);
        assert_eq!(payload["disposition"], "partial");
        assert_eq!(payload["rows"], 80);
        assert_eq!(payload["skipped_lines"], 0);
        assert_eq!(payload["grade_issues"][0]["raw_token"], "XX");
        assert_eq!(payload["grade_issues"][0]["kind"], "unrecognized");
        assert_eq!(payload["grade_issues"][0]["row"]["ordinal"], 25);
        assert_eq!(payload["grade_issues"][0]["row"]["byte_offset"], 1_000);
    }
}
