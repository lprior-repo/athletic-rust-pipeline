use census_domain::UsJurisdiction;
use census_reconcile::identity::{admitted_scope, WorkflowIdentity};
use restate_sdk::prelude::*;

use crate::restate_services::census::CensusClient;
use crate::restate_services::jobs;
use crate::restate_services::jurisdiction::JurisdictionCensusClient;
use crate::restate_services::wire::{
    BindRunRequest, JurisdictionOwed, JurisdictionReport, JurisdictionSummary, NationalFailure,
    NationalRequest,
};

use super::classification::{classify, Completion};

pub(super) async fn redrive(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
    targets: &[(UsJurisdiction, String)],
    jurisdictions: &mut Vec<JurisdictionSummary>,
    failures: &mut Vec<NationalFailure>,
) -> Result<Vec<JurisdictionOwed>, HandlerError> {
    let mut owed: Vec<JurisdictionOwed> = Vec::new();
    let mut pending: Vec<UsJurisdiction> = failures
        .iter()
        .map(|failure| failure.jurisdiction)
        .collect();
    let mut rounds: usize = 0;
    while !pending.is_empty() && rounds < request.pass_budget {
        rounds = rounds.saturating_add(1);
        ctx.sleep(std::time::Duration::from_secs(request.pass_delay_seconds))
            .await?;
        let (next, no_progress) = pass_round(ctx, request, targets, &pending, &mut owed).await?;
        for classified in classify_round(ctx, request, targets, no_progress).await? {
            apply_completion(jurisdictions, failures, &mut owed, classified);
        }
        pending = next;
        tracing::info!(
            round = rounds,
            owed = owed.len(),
            pending = pending.len(),
            "re-drove owed jurisdictions"
        );
    }
    Ok(owed)
}

pub(super) fn require_identity(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
) -> Result<WorkflowIdentity, HandlerError> {
    let identity =
        WorkflowIdentity::national(request.season, request.revision, &request.jurisdictions);
    if ctx.key() != identity.as_str() {
        return Err(TerminalError::new(format!(
            "request identity {} does not match workflow id {}",
            identity.as_str(),
            ctx.key()
        ))
        .into());
    }
    Ok(identity)
}

async fn pass_round(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
    targets: &[(UsJurisdiction, String)],
    pending: &[UsJurisdiction],
    owed: &mut Vec<JurisdictionOwed>,
) -> Result<(Vec<UsJurisdiction>, Vec<(UsJurisdiction, JurisdictionOwed)>), HandlerError> {
    let mut passes = DurableFuturesUnordered::new();
    for jurisdiction in pending {
        passes.push(
            ctx.object_client::<JurisdictionCensusClient>(target_key(targets, *jurisdiction)?)
                .pass(Json(request.for_jurisdiction(*jurisdiction)))
                .call(),
        );
    }
    let mut next: Vec<UsJurisdiction> = Vec::new();
    let mut no_progress: Vec<(UsJurisdiction, JurisdictionOwed)> = Vec::new();
    while let Some((index, outcome)) = passes.next().await? {
        let Some(jurisdiction) = pending.get(index).copied() else {
            return Err(jobs::invariant(
                "the re-drive reported an index it never pushed",
            ));
        };
        match outcome {
            Ok(Json(pass)) if !pass.owed.is_empty() && !pass.stages_run.is_empty() => {
                next.push(jurisdiction);
            }
            Ok(Json(pass)) => no_progress.push((jurisdiction, pass)),
            Err(error) => owed.push(unanswered(jurisdiction, targets, &error)?),
        }
    }
    Ok((next, no_progress))
}

fn unanswered<E: std::fmt::Display>(
    jurisdiction: UsJurisdiction,
    targets: &[(UsJurisdiction, String)],
    error: &E,
) -> Result<JurisdictionOwed, HandlerError> {
    Ok(JurisdictionOwed {
        identity: target_key(targets, jurisdiction)?,
        jurisdiction,
        stages_run: Vec::new(),
        owed: Vec::new(),
        reasons: vec![error.to_string()],
    })
}

async fn classify_round(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
    targets: &[(UsJurisdiction, String)],
    no_progress: Vec<(UsJurisdiction, JurisdictionOwed)>,
) -> Result<Vec<(UsJurisdiction, JurisdictionOwed, Completion)>, HandlerError> {
    let mut runs = DurableFuturesUnordered::new();
    for (jurisdiction, _) in &no_progress {
        runs.push(
            ctx.object_client::<JurisdictionCensusClient>(target_key(targets, *jurisdiction)?)
                .run(Json(request.for_jurisdiction(*jurisdiction)))
                .call(),
        );
    }
    let mut outcomes: Vec<Option<Result<Json<JurisdictionReport>, TerminalError>>> =
        no_progress.iter().map(|_| None).collect();
    while let Some((index, outcome)) = runs.next().await? {
        let Some(slot) = outcomes.get_mut(index) else {
            return Err(jobs::invariant(
                "the re-drive reported an index it never pushed",
            ));
        };
        *slot = Some(outcome);
    }
    classify_all(no_progress, outcomes, targets)
}

fn classify_all(
    no_progress: Vec<(UsJurisdiction, JurisdictionOwed)>,
    outcomes: Vec<Option<Result<Json<JurisdictionReport>, TerminalError>>>,
    targets: &[(UsJurisdiction, String)],
) -> Result<Vec<(UsJurisdiction, JurisdictionOwed, Completion)>, HandlerError> {
    let mut classified = Vec::with_capacity(no_progress.len());
    for ((jurisdiction, pass), outcome) in no_progress.into_iter().zip(outcomes) {
        let Some(outcome) = outcome else {
            return Err(jobs::invariant("the re-drive lost a jurisdiction outcome"));
        };
        let key = target_key(targets, jurisdiction)?;
        classified.push((jurisdiction, pass, classify(jurisdiction, &key, outcome)));
    }
    Ok(classified)
}

fn apply_completion(
    jurisdictions: &mut Vec<JurisdictionSummary>,
    failures: &mut Vec<NationalFailure>,
    owed: &mut Vec<JurisdictionOwed>,
    classified: (UsJurisdiction, JurisdictionOwed, Completion),
) {
    let (jurisdiction, pass, completion) = classified;
    match completion {
        Completion::Answered(summary) => {
            jurisdictions.retain(|existing| existing.jurisdiction != jurisdiction);
            failures.retain(|existing| existing.jurisdiction != jurisdiction);
            jurisdictions.push(summary);
        }
        Completion::Unanswered(failure) => {
            failures.retain(|existing| existing.jurisdiction != jurisdiction);
            failures.push(failure);
            owed.push(pass);
        }
    }
}

fn target_key(
    targets: &[(UsJurisdiction, String)],
    jurisdiction: UsJurisdiction,
) -> Result<String, HandlerError> {
    targets
        .iter()
        .find(|(candidate, _)| *candidate == jurisdiction)
        .map(|(_, key)| key.clone())
        .ok_or_else(|| jobs::invariant("a re-driven jurisdiction is not a run target"))
}

pub(super) async fn bind_run(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
) -> Result<(), HandlerError> {
    let jurisdictions = admitted_scope(&request.jurisdictions);
    let Json(_reply) = ctx
        .service_client::<CensusClient>()
        .bind_run(Json(BindRunRequest {
            season: request.season.get(),
            revision: request.revision.get(),
            jurisdictions,
        }))
        .call()
        .await?;
    Ok(())
}
