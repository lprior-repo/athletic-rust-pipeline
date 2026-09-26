//! Deterministic reconciliation of the athlete rows one provider object spans.
//!
//! The merge keys an athlete on the four facts its id is minted from — school, normalized name,
//! graduating class, gender side — and the athlete-identity family only compares two rows that key
//! brings together. Both leave findings a store-wide read settles without asking a model:
//!
//! * one provider object (one `(namespace, source athlete id)`) named by two canonical rows whose
//!   schools differ: an athlete who transferred. The rows are one person when their name, class and
//!   gender agree, and that is a rule rather than a judgement, so the pass decides it.
//! * one canonical row carrying two objects of one namespace: the merge may have folded two athletes
//!   into a row the provider never said was one. Which of the two the row is cannot be read off the
//!   store, so the finding is filed and left pending for the lane or an operator.
//!
//! A verdict is evidence, not an edit: this pass writes cases and verdicts and never rewrites a
//! canonical row, and it leaves any case a decision already stands on exactly as it found it.
//!
//! The store as this pass sees it, and the two findings that read off it, are in
//! [`crate::athlete_cluster_findings`]: what is left here is the decision and the write.

use std::collections::BTreeSet;

use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{Store, StoreResult, Table};

use crate::athlete_cluster_findings::Alias;
use crate::athlete_cluster_findings::Observed;

/// The reviewer a rule-written verdict is filed under.
///
/// A deterministic rule and a model are different evidence, so they are never filed under one name:
/// a pass that decided by rule is readable in the verdict table as exactly that.
pub const RULE_REVIEWER: &str = "deterministic:shared-provider-object";

/// What one reconciliation pass did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileReport {
    /// Canonical athlete rows read.
    pub rows: usize,
    /// Provider objects the rows name.
    pub objects: usize,
    /// Cases this pass filed.
    pub filed: usize,
    /// Cases this pass decided by rule.
    pub decided: usize,
    /// Cases this pass filed and left pending.
    pub pending: usize,
    /// Rows carrying more than one object of one namespace.
    pub aliases: usize,
    /// Findings a standing decision already covers, left untouched.
    pub held: usize,
}

impl ReconcileReport {
    /// One line an operator reads after a pass.
    pub fn summary(&self) -> String {
        format!(
            "{} athlete rows, {} provider objects, {} cases filed ({} decided, {} pending), {} rows holding several objects of one provider, {} findings left to a standing decision",
            self.rows,
            self.objects,
            self.filed,
            self.decided,
            self.pending,
            self.aliases,
            self.held
        )
    }
}

/// File alias cases: each multi-object row that no standing decision already covers.
fn file_alias_cases(
    aliases: &[Alias],
    standing: &BTreeSet<String>,
    report: &mut ReconcileReport,
    cases: &mut Vec<ReviewCase>,
) {
    for alias in aliases {
        let Some(case) = alias.case() else {
            continue;
        };
        if standing.contains(&case.id) {
            report.held = report.held.saturating_add(1);
            continue;
        }
        report.pending = report.pending.saturating_add(1);
        report.filed = report.filed.saturating_add(1);
        cases.push(case);
    }
}

/// Write the identity findings a store-wide read of provider objects states.
pub fn reconcile_athletes(
    store: &Store,
    observed_at: &str,
    dry_run: bool,
) -> StoreResult<ReconcileReport> {
    let observed = Observed::read(store)?;
    let standing = standing_cases(store)?;
    let aliases = observed.aliases();
    let mut report = ReconcileReport {
        rows: observed.rows.len(),
        objects: observed.by_object.len(),
        aliases: aliases.len(),
        ..ReconcileReport::default()
    };
    let mut cases: Vec<ReviewCase> = Vec::new();
    let mut verdicts: Vec<ReviewVerdictRecord> = Vec::new();
    for span in observed.spans() {
        let Some(case) = span.case() else {
            continue;
        };
        if standing.contains(&case.id) {
            report.held = report.held.saturating_add(1);
            continue;
        }
        if span.hard_contradiction().is_some() {
            report.pending = report.pending.saturating_add(1);
            cases.push(case);
        } else if span.agrees() {
            let mut settled = case;
            settled.state = ReviewState::Resolved;
            report.decided = report.decided.saturating_add(1);
            verdicts.push(span.verdict(&settled, observed_at));
            cases.push(settled);
        } else {
            report.pending = report.pending.saturating_add(1);
            cases.push(case);
        }
        report.filed = report.filed.saturating_add(1);
    }
    file_alias_cases(&aliases, &standing, &mut report, &mut cases);
    if !dry_run && (!cases.is_empty() || !verdicts.is_empty()) {
        let mut batch = store.write_batch();
        batch.replace_many(Table::ReviewCases, &cases)?;
        batch.replace_many(Table::IdentityVerdicts, &verdicts)?;
        let digest = super::compute_digest(&verdicts, &cases);
        let operation = format!("reconcile:{observed_at}:{digest}");
        batch.commit_once(&operation, &digest)?;
    }
    Ok(report)
}

/// The cases a decision already stands on: a pass neither rewrites the row nor the verdict under it.
fn standing_cases(store: &Store) -> StoreResult<BTreeSet<String>> {
    Ok(store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .filter(|case| case.state != ReviewState::Pending)
        .map(|case| case.id)
        .collect())
}

#[cfg(test)]
#[path = "athlete_clusters_tests.rs"]
mod tests;
