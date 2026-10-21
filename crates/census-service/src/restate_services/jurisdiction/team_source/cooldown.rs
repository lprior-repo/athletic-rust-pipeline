use super::execution_ledger::ExecutionLedger;
#[cfg(test)]
use super::ledger;
use super::TeamsSource;
use crate::restate_services::support::{blocking, job_error, JobError};
use crate::restate_services::wire::{TeamsSourceOutcome, TeamsSourceRequest};
use census_domain::model::SourceAccessCondition;
use census_store::{clock::Clock, Store, Table};
use chrono::{DateTime, Utc};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::sync::Arc;
use std::time::Duration;

const MAX_DEFERRAL_ROUNDS: u8 = 3;

#[derive(Serialize, Deserialize)]
enum GateDecision {
    Proceed,
    Settled(TeamsSourceOutcome),
    Sleep { milliseconds: u64, message: String },
    Blocked { message: String },
}

impl TeamsSource {
    #[tracing::instrument(skip_all)]
    pub(super) async fn wait_recorded_access(
        &self,
        ctx: &ObjectContext<'_>,
        request: &Arc<TeamsSourceRequest>,
        scope: &ExecutionLedger,
    ) -> Result<Option<TeamsSourceOutcome>, HandlerError> {
        for round in 1..=MAX_DEFERRAL_ROUNDS {
            match self.recorded_access_round(ctx, request, scope).await? {
                GateDecision::Proceed => return Ok(None),
                GateDecision::Settled(outcome) => return Ok(Some(outcome)),
                GateDecision::Blocked { message } => return Err(deferred(message)),
                GateDecision::Sleep {
                    milliseconds,
                    message,
                } => {
                    if round == MAX_DEFERRAL_ROUNDS {
                        return Err(deferred(format!(
                            "{message}; stopped after three checks and two durable sleeps"
                        )));
                    }
                    ctx.sleep(Duration::from_millis(milliseconds)).await?;
                }
            }
        }
        Err(deferred(
            "source access decision bound reached without admission".into(),
        ))
    }

    #[tracing::instrument(skip_all)]
    async fn recorded_access_round(
        &self,
        ctx: &ObjectContext<'_>,
        request: &Arc<TeamsSourceRequest>,
        scope: &ExecutionLedger,
    ) -> Result<GateDecision, HandlerError> {
        let store = Arc::clone(&self.owner.store);
        let clock = Arc::clone(&self.owner.clock);
        let region = Arc::clone(self.jobs.region());
        let request = Arc::clone(request);
        let operation = ctx.key().to_string();
        let scope = scope.clone();
        let Json(decision) = ctx
            .run(move || async move {
                blocking(region, move || {
                    read_decision_in(&store, clock.as_ref(), &operation, &request, &scope)
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(decision)
    }
}

#[cfg(test)]
fn read_decision(
    store: &Store,
    clock: &dyn Clock,
    operation: &str,
    request: &TeamsSourceRequest,
) -> Result<GateDecision, JobError> {
    read_decision_in(store, clock, operation, request, &ExecutionLedger::Original)
}

fn read_decision_in(
    store: &Store,
    clock: &dyn Clock,
    operation: &str,
    request: &TeamsSourceRequest,
    scope: &ExecutionLedger,
) -> Result<GateDecision, JobError> {
    if let Some(outcome) = scope.status(store, operation, Some(request))? {
        return Ok(GateDecision::Settled(outcome));
    }
    let Some(source) = census_crawl::registry::descriptor(&request.source) else {
        return Ok(GateDecision::Proceed);
    };
    let conditions = store.scan::<SourceAccessCondition>(Table::SourceAccess)?;
    let now = clock.today_iso8601();
    Ok(gate_decision(
        &conditions,
        &now,
        source.admission.origin,
        &request.source,
    ))
}

fn gate_decision(
    conditions: &[SourceAccessCondition],
    now: &str,
    host: &str,
    source: &str,
) -> GateDecision {
    let Some(condition) = active_block(conditions, now, host) else {
        return GateDecision::Proceed;
    };
    let message = format!(
        "source {source} deferred by retained {} condition {} on {} (HTTP {}, deadline {:?}); no acquisition attempted or attempt reserved",
        condition.kind.slug(), condition.id, condition.host, condition.status, condition.cooldown_until
    );
    match sleep_milliseconds(condition, now) {
        Some(milliseconds) => GateDecision::Sleep {
            milliseconds,
            message,
        },
        None => GateDecision::Blocked { message },
    }
}

fn active_block<'a>(
    conditions: &'a [SourceAccessCondition],
    now: &str,
    host: &str,
) -> Option<&'a SourceAccessCondition> {
    conditions
        .iter()
        .filter(|condition| condition.host.eq_ignore_ascii_case(host) && condition.is_blocking(now))
        .max_by_key(|condition| {
            let deadline = deadline(condition);
            (deadline.is_none(), deadline, Reverse(condition.id.as_str()))
        })
}

fn deadline(condition: &SourceAccessCondition) -> Option<DateTime<Utc>> {
    condition.cooldown_until.as_deref()?.parse().ok()
}

fn sleep_milliseconds(condition: &SourceAccessCondition, now: &str) -> Option<u64> {
    let now = now.parse::<DateTime<Utc>>().ok()?;
    let delta = deadline(condition)?.signed_duration_since(now);
    let milliseconds = u64::try_from(delta.num_milliseconds()).ok()?;
    if milliseconds > 0 {
        Some(milliseconds)
    } else if delta > chrono::TimeDelta::zero() {
        Some(1)
    } else {
        None
    }
}

fn deferred(message: String) -> HandlerError {
    job_error(JobError::Transient { message })
}

#[cfg(test)]
#[path = "cooldown/selection_tests.rs"]
mod selection_tests;
