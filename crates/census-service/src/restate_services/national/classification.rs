use census_domain::UsJurisdiction;
use restate_sdk::prelude::*;

use crate::restate_services::wire::{JurisdictionReport, NationalRequest};
use crate::restate_services::{jobs, KEY_STATE};
use census_domain::national_census::{JurisdictionSummary, NationalFailure};

use super::report::assemble_partial;

pub(in crate::restate_services) enum Completion {
    Answered(JurisdictionSummary),
    Unanswered(NationalFailure),
}

pub(in crate::restate_services) fn classify(
    jurisdiction: UsJurisdiction,
    key: &str,
    outcome: Result<Json<JurisdictionReport>, TerminalError>,
) -> Completion {
    match outcome {
        Ok(Json(report)) => Completion::Answered(JurisdictionSummary {
            jurisdiction,
            identity: report.identity,
            rosters_total: report.rosters.rosters_total,
            rosters_committed: report.rosters.rosters_committed,
            rosters_remaining: report.rosters.rosters_remaining,
            rosters_skipped: report.rosters.rosters_skipped,
            blocked: report.rosters.blocked,
            athletes: report.rosters.athletes,
            class_of_2027: report.rosters.class_of_2027,
        }),
        Err(error) => Completion::Unanswered(NationalFailure {
            jurisdiction,
            identity: key.to_string(),
            error: error.to_string(),
        }),
    }
}

pub(in crate::restate_services) async fn collect_outcomes(
    ctx: &WorkflowContext<'_>,
    request: &NationalRequest,
    in_flight: &mut DurableFuturesUnordered<impl CallFuture<Response = Json<JurisdictionReport>>>,
    targets: &[(UsJurisdiction, String)],
) -> Result<(Vec<JurisdictionSummary>, Vec<NationalFailure>), HandlerError> {
    let mut summaries: Vec<JurisdictionSummary> = Vec::with_capacity(targets.len());
    let mut failures: Vec<NationalFailure> = Vec::new();
    while let Some((index, outcome)) = in_flight.next().await? {
        let Some((jurisdiction, key)) = targets.get(index) else {
            return Err(jobs::invariant(
                "the fan-out reported an index it never pushed",
            ));
        };
        match classify(*jurisdiction, key, outcome) {
            Completion::Answered(summary) => summaries.push(summary),
            Completion::Unanswered(failure) => failures.push(failure),
        }
        let partial =
            assemble_partial(request.season, request.revision, &summaries, &failures, &[])
                .map_err(|error| TerminalError::new(error.to_string()))?;
        ctx.set(KEY_STATE, Json(partial));
    }
    Ok((summaries, failures))
}
