mod failures;

use crate::{
    model::SourceRecord,
    runtime::{
        acquisition::{QueryEvidence, QueryPage},
        protocol::SourceResource,
        row_protocol::DiscoverySummary,
        row_worker::MAX_CANDIDATES,
    },
    search::{parse_page, query_plan, SearchPage, SearchProgress},
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::BTreeSet;
use url::Url;
#[derive(Debug)]
struct VerifiedQuery {
    candidate_ids: Vec<crate::domain::identity::AthleteId>,
    issues: Vec<String>,
    complete: bool,
}
pub(super) fn verify(
    source: &SourceRecord,
    discovery: &DiscoverySummary,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<()> {
    let planned = query_plan(source).context("recomputing deterministic search plan")?;
    if planned.is_empty() {
        bail!("source search plan is empty");
    }
    let (verified, issues, missing) = verify_query_sequence(&planned, discovery, origin, store)?;
    let (candidate_ids, candidate_limit) = candidate_union(&verified);
    if candidate_ids != discovery.candidate_ids {
        bail!("discovery candidate IDs differ from the retained query-page union");
    }
    if issues != discovery.issues {
        bail!("discovery issues differ from retained query evidence");
    }
    let complete = !missing
        && !candidate_limit
        && issues.is_empty()
        && verified.iter().all(|query| query.complete);
    if discovery.complete != complete {
        bail!("discovery completeness differs from retained query evidence");
    }
    Ok(())
}
fn verify_query_sequence(
    planned: &[crate::search::SearchQuery],
    discovery: &DiscoverySummary,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<(Vec<VerifiedQuery>, Vec<String>, bool)> {
    let mut verified = Vec::new();
    let mut issues = Vec::new();
    let mut issue_index = 0_usize;
    let mut artifact_index = 0_usize;
    let mut missing = false;
    let identities = planned
        .iter()
        .map(|query| query.cache_identity())
        .collect::<Vec<_>>();
    planned.iter().enumerate().try_for_each(|(current, _)| {
        let Some(digest) = discovery.query_artifacts.get(artifact_index) else {
            consume_missing_issue(&discovery.issues, &mut issue_index, &mut issues)?;
            missing = true;
            return Ok(());
        };
        let evidence: QueryEvidence = load_json(digest, store, "query evidence")?;
        let identity = evidence.query.cache_identity();
        let Some(index) = identities
            .iter()
            .position(|candidate| candidate == &identity)
        else {
            bail!("query evidence is not present in the deterministic search plan");
        };
        if index < current {
            bail!("query artifacts are duplicated or out of order");
        }
        if index > current {
            consume_missing_issue(&discovery.issues, &mut issue_index, &mut issues)?;
            missing = true;
            return Ok(());
        }
        let result = verify_query(&evidence, origin, store)?;
        consume_verified_issues(
            &result.issues,
            &discovery.issues,
            &mut issue_index,
            &mut issues,
        )?;
        verified.push(result);
        artifact_index = artifact_index.saturating_add(1);
        Ok(())
    })?;
    if artifact_index != discovery.query_artifacts.len() {
        bail!("discovery retains an extra or reordered query artifact");
    }
    if issue_index != discovery.issues.len() {
        bail!("discovery contains an unaccounted query issue");
    }
    Ok((verified, issues, missing))
}
fn consume_missing_issue(
    retained: &[String],
    index: &mut usize,
    issues: &mut Vec<String>,
) -> Result<()> {
    let issue = retained
        .get(*index)
        .context("missing query artifact has no producer failure issue")?;
    if !issue.starts_with("query worker call failed: ") {
        bail!("missing query artifact is not explained by a query worker call failure");
    }
    issues.push(issue.clone());
    *index = (*index).saturating_add(1);
    Ok(())
}
fn consume_verified_issues(
    expected: &[String],
    retained: &[String],
    index: &mut usize,
    issues: &mut Vec<String>,
) -> Result<()> {
    expected.iter().try_for_each(|issue| {
        if retained.get(*index) != Some(issue) {
            bail!("discovery issues differ from retained query evidence");
        }
        issues.push(issue.clone());
        *index = (*index).saturating_add(1);
        Ok(())
    })
}
fn verify_query(
    evidence: &QueryEvidence,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<VerifiedQuery> {
    // QueryWorker shares results by case-insensitive text and sport, not planning
    // stage. Verify the actual cached request after binding its identity to the plan.
    let mut progress = SearchProgress::new(evidence.query.clone());
    let mut candidate_ids = Vec::new();
    let mut reconciliation_start = None;
    evidence
        .pages
        .iter()
        .enumerate()
        .try_for_each(|(index, page)| {
            let expected_start = progress
                .next_offset()
                .context("query evidence contains a page after pagination completed")?;
            let parsed = verify_page(&evidence.query, page, expected_start, origin, store)?;
            candidate_ids.extend(parsed.results().iter().map(|candidate| candidate.id()));
            if reconciliation_start.is_some() {
                bail!("query evidence retains a page after pagination failure");
            }
            match progress.consume(parsed) {
                Ok(()) => Ok(()),
                Err(_) => {
                    reconciliation_start = Some(expected_start);
                    if index + 1 != evidence.pages.len() {
                        bail!("query evidence retains a page after reconciliation failure");
                    }
                    Ok::<(), anyhow::Error>(())
                }
            }
        })?;
    failures::verify(
        evidence,
        progress.next_offset(),
        reconciliation_start,
        origin,
        store,
    )?;
    let progress_complete = progress.complete();
    let progress_issues = progress.into_issues();
    if progress_issues != evidence.issues {
        bail!("query issues differ from replayed SearchProgress state");
    }
    if !evidence.complete && evidence.failures.is_empty() && evidence.issues.is_empty() {
        bail!("incomplete query evidence has no failure or parser issue");
    }
    let expected_complete =
        evidence.failures.is_empty() && reconciliation_start.is_none() && progress_complete;
    if evidence.complete != expected_complete {
        bail!("query completeness differs from replayed pagination state");
    }
    let issues = evidence
        .issues
        .iter()
        .map(|issue| format!("query issue {}: {}", issue.code, issue.message))
        .chain(
            evidence
                .failures
                .iter()
                .map(|failure| format!("query failure {:?}: {}", failure.code, failure.message)),
        )
        .collect();
    Ok(VerifiedQuery {
        candidate_ids,
        issues,
        complete: evidence.complete,
    })
}
fn verify_page(
    query: &crate::search::SearchQuery,
    page: &QueryPage,
    expected_start: u32,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<SearchPage> {
    let resource = SourceResource::Search {
        query: query.text().to_owned(),
        sport: query.sport(),
        start: expected_start,
    };
    super::source_receipts::verify_operation(
        &resource,
        &page.retries,
        &page.response,
        &page.previous_responses,
        origin,
        store,
    )
    .context("verifying captured search operation")?;
    let parsed: SearchPage = load_json(&page.parsed, store, "search page")?;
    if parsed.query != *query {
        bail!("search page query differs from its query evidence");
    }
    if parsed.start != expected_start {
        bail!("search page start differs from the planned request offset");
    }
    let raw = store
        .get_bytes(&page.response.digest)
        .context("reading retained raw search response")?;
    if u64::try_from(raw.len())? != page.response.bytes {
        bail!("search receipt byte count differs from retained raw bytes");
    }
    let reparsed = parse_page(query, expected_start, page.response.digest.clone(), &raw)
        .context("replaying retained raw search response")?;
    if reparsed != parsed {
        bail!("search page differs from replayed raw search response");
    }
    Ok(parsed)
}
fn load_json<T: DeserializeOwned + Serialize>(
    digest: &crate::domain::identity::EvidenceDigest,
    store: &ArtifactStore,
    name: &str,
) -> Result<T> {
    let bytes = store
        .get_bytes(digest)
        .with_context(|| format!("reading {name}"))?;
    let value = serde_json::from_slice(&bytes).with_context(|| format!("decoding {name}"))?;
    let encoded = serde_json::to_vec(&value).with_context(|| format!("serializing {name}"))?;
    if encoded != bytes {
        bail!("{name} is not the exact retained typed representation");
    }
    Ok(value)
}
fn candidate_union(
    queries: &[VerifiedQuery],
) -> (BTreeSet<crate::domain::identity::AthleteId>, bool) {
    queries
        .iter()
        .flat_map(|query| query.candidate_ids.iter().copied())
        .fold((BTreeSet::new(), false), |(mut ids, limited), id| {
            if limited || ids.contains(&id) {
                return (ids, limited);
            }
            if ids.len() >= MAX_CANDIDATES {
                return (ids, true);
            }
            ids.insert(id);
            (ids, false)
        })
}
