use std::collections::BTreeSet;

use census_domain::model::{AthleteIdentityIndex, ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{Application, Store, StoreResult, Table};

use crate::athlete_cluster_findings::Alias;
use crate::athlete_cluster_findings::Observed;

pub const RULE_REVIEWER: &str = "deterministic:shared-provider-object";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileReport {
    pub rows: usize,
    pub objects: usize,
    pub filed: usize,
    pub decided: usize,
    pub pending: usize,
    pub aliases: usize,
    pub held: usize,
}

impl ReconcileReport {
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

fn file_alias_cases(
    aliases: &[Alias<'_>],
    standing: &BTreeSet<String>,
    report: &mut ReconcileReport,
    index: &AthleteIdentityIndex,
    cases: &mut Vec<ReviewCase>,
) -> StoreResult<()> {
    for alias in aliases {
        let Some(case) = bound_case(alias.case(), index)? else {
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
    Ok(())
}

pub fn reconcile_athletes(
    store: &Store,
    observed_at: &str,
    dry_run: bool,
) -> StoreResult<ReconcileReport> {
    let observed = Observed::read(store)?;
    let mut identity_index = AthleteIdentityIndex::default();
    for athlete in &observed.rows {
        identity_index.observe(athlete)?;
    }
    let standing = standing_cases(store)?;
    let findings = observed.findings();
    let mut report = ReconcileReport {
        rows: observed.rows.len(),
        objects: findings.objects,
        aliases: findings.aliases.len(),
        ..ReconcileReport::default()
    };
    let mut cases: Vec<ReviewCase> = Vec::new();
    let mut verdicts: Vec<ReviewVerdictRecord> = Vec::new();
    for span in findings.spans {
        let Some(case) = bound_case(span.case(), &identity_index)? else {
            continue;
        };
        if standing.contains(&case.id) {
            report.held = report.held.saturating_add(1);
            continue;
        }
        if span.hard_contradiction().is_some() {
            report.pending = report.pending.saturating_add(1);
            cases.push(case);
        } else if span.agrees()
            && identity_index.supports_identity(
                census_domain::model::AppliedIdentityKind::SamePerson,
                &case.member_ids,
            )
        {
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
    file_alias_cases(
        &findings.aliases,
        &standing,
        &mut report,
        &identity_index,
        &mut cases,
    )?;
    if !dry_run && (!cases.is_empty() || !verdicts.is_empty()) {
        commit_cases(store, &cases, &verdicts, observed_at)?;
    }
    Ok(report)
}

fn commit_cases(
    store: &Store,
    cases: &[ReviewCase],
    verdicts: &[ReviewVerdictRecord],
    observed_at: &str,
) -> StoreResult<()> {
    let mut batch = store.write_batch();
    batch.replace_many(Table::ReviewCases, cases)?;
    batch.replace_many(Table::IdentityVerdicts, verdicts)?;
    let digest = super::compute_digest(verdicts, cases)?;
    let operation = format!("reconcile:{observed_at}:{digest}");
    let application = batch.commit_once(&operation, &digest)?;
    if matches!(application, Application::Repeated(_)) {
        crate::review_checkpoint::verify_repeated(store, verdicts, cases)?;
    }
    Ok(())
}

fn standing_cases(store: &Store) -> StoreResult<BTreeSet<String>> {
    Ok(store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .filter(|case| case.state != ReviewState::Pending)
        .map(|case| case.id)
        .collect())
}

fn bound_case(
    candidate: Option<ReviewCase>,
    index: &AthleteIdentityIndex,
) -> StoreResult<Option<ReviewCase>> {
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    let evidence =
        index.case_evidence(&candidate.subject, &candidate.detail, &candidate.member_ids)?;
    let mut current = ReviewCase::pending_with_evidence(
        &candidate.family,
        &candidate.subject_id,
        candidate.subject,
        candidate.detail,
        evidence,
    );
    current.member_ids = candidate.member_ids;
    Ok(Some(current))
}

#[cfg(test)]
#[path = "athlete_clusters_tests.rs"]
mod tests;
