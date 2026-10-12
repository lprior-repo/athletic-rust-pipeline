use super::super::jurisdiction::{team_source::contacts_key, TeamsSourceClient};
use super::super::wire::{
    ContactStatus, ContactSummary, NationalRequest, SourceStageOutcome, StageOutcome,
    TeamsAttemptProgress, TeamsSourceOutcome, TeamsSourceRequest,
};
use census_domain::UsJurisdiction;
use restate_sdk::prelude::*;

pub(super) const KEY_CONTACTS: &str = "school_contacts";

pub(super) async fn collect(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
    targets: &[(UsJurisdiction, String)],
    at: &str,
) -> Result<Vec<ContactSummary>, HandlerError> {
    let mut in_flight = DurableFuturesUnordered::new();
    for (jurisdiction, identity) in targets {
        let input = TeamsSourceRequest {
            jurisdiction: request.for_jurisdiction(*jurisdiction),
            source: census_crawl::sidearm_staff::SOURCE_ID.to_owned(),
            observed_on: at.to_owned(),
        };
        in_flight.push(
            ctx.object_client::<TeamsSourceClient>(contacts_key(identity))
                .contacts(Json(input))
                .call(),
        );
    }
    collect_outcomes(in_flight, targets, at).await
}

async fn collect_outcomes(
    mut in_flight: DurableFuturesUnordered<impl CallFuture<Response = Json<TeamsSourceOutcome>>>,
    targets: &[(UsJurisdiction, String)],
    at: &str,
) -> Result<Vec<ContactSummary>, HandlerError> {
    let mut summaries = Vec::with_capacity(targets.len());
    while let Some((index, answer)) = in_flight.next().await? {
        let (jurisdiction, identity) = targets
            .get(index)
            .ok_or_else(|| super::jobs::invariant("contact fan-out returned an unknown target"))?;
        let identity = contacts_key(identity);
        match answer {
            Ok(Json(outcome)) => summaries.push(summarize(*jurisdiction, identity, outcome, at)),
            Err(error) => {
                tracing::warn!(%error, identity, "school contact handler failed with owed work");
                summaries.push(ContactSummary {
                    jurisdiction: *jurisdiction,
                    identity: identity.clone(),
                    status: ContactStatus::HandlerFailed,
                    source_rows: None,
                    errors: None,
                    unfinished: None,
                    sources: source_outcomes(
                        *jurisdiction,
                        Some(&StageOutcome {
                            records: 0,
                            at: at.into(),
                            errors: vec![error.to_string()],
                            notes: Vec::new(),
                            disposition: census_crawl::CollectionDisposition::Failed,
                            unfinished: vec![identity.clone()],
                        }),
                        at,
                    ),
                });
            }
        }
    }
    summaries.sort_by_key(|summary| summary.jurisdiction.code());
    Ok(summaries)
}

fn summarize(
    jurisdiction: UsJurisdiction,
    identity: String,
    answer: TeamsSourceOutcome,
    at: &str,
) -> ContactSummary {
    let (status, outcome) = match &answer {
        TeamsSourceOutcome::Completed { outcome, .. } => {
            let status = if outcome.disposition.is_complete()
                && outcome.errors.is_empty()
                && outcome.unfinished.is_empty()
            {
                ContactStatus::Complete
            } else {
                ContactStatus::Partial
            };
            (status, Some(outcome))
        }
        TeamsSourceOutcome::Terminal { progress, .. } => {
            (ContactStatus::Terminal, latest(progress))
        }
        TeamsSourceOutcome::Exhausted { progress, .. } => {
            (ContactStatus::Exhausted, latest(progress))
        }
        TeamsSourceOutcome::Interrupted { progress, .. } => {
            (ContactStatus::Interrupted, latest(progress))
        }
    };
    ContactSummary {
        jurisdiction,
        identity,
        status,
        source_rows: outcome
            .filter(|outcome| outcome.disposition != census_crawl::CollectionDisposition::Unknown)
            .map(|outcome| outcome.records),
        errors: outcome.map(|outcome| outcome.errors.len()),
        unfinished: outcome.map(|outcome| outcome.unfinished.len()),
        sources: source_outcomes(jurisdiction, outcome, at),
    }
}

fn latest(progress: &[TeamsAttemptProgress]) -> Option<&StageOutcome> {
    progress.iter().rev().find_map(|step| match step {
        TeamsAttemptProgress::Completed { outcome, .. } => Some(outcome),
        TeamsAttemptProgress::Transient { outcome, .. }
        | TeamsAttemptProgress::Terminal { outcome, .. } => outcome.as_ref(),
        TeamsAttemptProgress::Unknown { .. } => None,
    })
}

fn source_outcomes(
    jurisdiction: UsJurisdiction,
    staff: Option<&StageOutcome>,
    at: &str,
) -> Vec<SourceStageOutcome> {
    let mut sources = Vec::with_capacity(2);
    if let Some(outcome) = staff {
        sources.push(SourceStageOutcome {
            slug: census_crawl::sidearm_staff::SOURCE_ID.into(),
            outcome: outcome.clone(),
        });
    }
    if census_crawl::applicability::applicable_sources(jurisdiction)
        .iter()
        .any(|source| source.slug == "coach_contacts")
    {
        sources.push(missing_import(at));
    }
    sources
}

fn missing_import(at: &str) -> SourceStageOutcome {
    SourceStageOutcome {
        slug: "coach_contacts".into(),
        outcome: StageOutcome {
            records: 0,
            at: at.into(),
            errors: Vec::new(),
            notes: vec!["prerequisite missing: no qualified coach_contacts import artifact is bound to this run; the existing import_csv collector and verified-artifact reader require an operator-supplied public contact artifact with retained evidence; school-site research does not complete this import lane".into()],
            disposition: census_crawl::CollectionDisposition::Blocked,
            unfinished: vec!["coach_contacts:verified-import-artifact".into()],
        },
    }
}

#[cfg(test)]
mod tests;
