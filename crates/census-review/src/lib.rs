#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

mod ask;
mod athlete_cluster_findings;
mod athlete_clusters;
pub mod athlete_flags;
mod athlete_packet;
mod athlete_verdict;
mod cohort_evidence;
mod consensus;
mod families;
mod model;
mod packets;
mod records;
mod review_budget;
mod review_checkpoint;
mod review_intake;
mod review_process;
mod review_subjects;
mod revocation;
mod verdicts;

use census_domain::model::{ReviewCase, ReviewVerdictRecord};
use futures::stream::{self, TryStreamExt};
use tracing::info;

use census_store::{Store, StoreResult};
use consensus::independent;

pub use athlete_clusters::{reconcile_athletes, ReconcileReport, RULE_REVIEWER};
pub use athlete_verdict::AthleteVerdict;
pub use families::{ReviewFamily, ReviewOptions};
pub use model::{ModelClient, ModelError, ModelOptions, ModelResponseFormat};
pub use records::ReviewReport;
pub use verdicts::{triage, validate, Adjudication, Admitted, Refusal};

const CHECKPOINT_CASES: usize = 256;

pub fn validate_lanes(clients: &[ModelClient]) -> StoreResult<()> {
    independent(clients).map(|_| ())
}

pub(crate) fn compute_digest(
    verdicts: &[ReviewVerdictRecord],
    cases: &[ReviewCase],
) -> StoreResult<String> {
    census_domain::model::serialized_digest(&(verdicts, cases)).map_err(|source| {
        census_store::StoreError::Invariant {
            detail: format!("cannot hash review checkpoint: {source}"),
        }
    })
}

pub async fn run_lanes(
    store: &Store,
    clients: &[ModelClient],
    options: &ReviewOptions,
    observed_at: &str,
) -> StoreResult<ReviewReport> {
    let clients = independent(clients)?;
    let original_snapshot = store.snapshot();
    let cache_snapshot = &original_snapshot;
    if !options.dry_run {
        revocation::invalidate(store, options, clients, observed_at).await?;
    }
    if options.limit == 0 {
        return Ok(ReviewReport::default());
    }
    let checkpoints = stream::try_unfold(
        (None::<String>, 0_usize, ReviewReport::default()),
        |(after, index, mut report)| async move {
            if report.requested >= options.limit {
                return Ok::<_, census_store::StoreError>(None);
            }
            let intake = review_intake::next(store, options, after.as_deref())?;
            let Some(cursor) = intake.cursor else {
                return Ok(None);
            };
            if !intake.cases.is_empty() {
                review_process::process(
                    store,
                    &intake.cases,
                    clients,
                    cache_snapshot,
                    options,
                    review_process::Checkpoint { observed_at, index },
                    &mut report,
                )
                .await?;
            }
            let next = index
                .checked_add(1)
                .ok_or(census_store::StoreError::CounterOverflow)?;
            Ok(Some((report.clone(), (Some(cursor), next, report))))
        },
    );
    let report = checkpoints
        .try_fold(ReviewReport::default(), |_, report| {
            futures::future::ready(Ok(report))
        })
        .await?;
    info!(summary = %report.summary(), "review pass finished");
    Ok(report)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "athlete_tests.rs"]
mod athlete_tests;

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod checkpoint_tests;

#[cfg(test)]
mod membership_tests;
