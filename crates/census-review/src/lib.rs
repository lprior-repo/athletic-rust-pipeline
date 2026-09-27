
#![forbid(unsafe_code)]

mod ask;
mod athlete_cluster_findings;
mod athlete_clusters;
pub mod athlete_flags;
mod athlete_packet;
mod athlete_verdict;
mod families;
mod model;
mod packets;
mod records;
mod verdicts;

use census_domain::model::ReviewCase;
use futures::stream::{self, StreamExt};
use tracing::{info, warn};

use census_store::{Application, Store, StoreResult, Table};

use ask::{ask_case, Answer};
use packets::{pending_cases, SubjectIndex};
use records::record_case;

pub use athlete_clusters::{reconcile_athletes, ReconcileReport, RULE_REVIEWER};
pub use athlete_verdict::AthleteVerdict;
pub use families::{ReviewFamily, ReviewOptions};
pub use model::{ModelClient, ModelError, ModelOptions};
pub use records::ReviewReport;
pub use verdicts::{triage, validate, Adjudication, Admitted, Refusal};

const CHECKPOINT_CASES: usize = 256;

pub async fn run(
    store: &Store,
    client: &ModelClient,
    options: &ReviewOptions,
    observed_at: &str,
) -> StoreResult<ReviewReport> {
    run_lanes(store, std::slice::from_ref(client), options, observed_at).await
}

async fn ask_lanes<'a>(
    pending: &[(ReviewCase, ReviewFamily)],
    subjects: &SubjectIndex,
    clients: &'a [ModelClient],
) -> Vec<(usize, &'a str, Answer)> {
    let lanes = clients.len();
    let roster: Vec<&ModelClient> = pending
        .iter()
        .enumerate()
        .filter_map(|(index, _)| clients.get(index.checked_rem(lanes)?))
        .collect();
    stream::iter(pending.iter().zip(roster))
        .enumerate()
        .map(|(index, ((case, family), client))| async move {
            let answer = ask_case(client, subjects, case, *family).await;
            (index, client.options().model_name(), answer)
        })
        .buffered(lanes)
        .collect()
        .await
}

fn process_answers(
    pending: &[(ReviewCase, ReviewFamily)],
    asked: Vec<(usize, &str, Answer)>,
    observed_at: &str,
    report: &mut ReviewReport,
) -> StoreResult<(
    Vec<census_domain::model::ReviewVerdictRecord>,
    Vec<ReviewCase>,
)> {
    let mut verdicts = Vec::new();
    let mut closed = Vec::new();
    for (index, reviewer, answer) in asked {
        let Some((case, _)) = pending.get(index) else {
            continue;
        };
        match answer {
            Answer::NoSubject => {
                warn!(case = %case.id, "retained case names a subject the store does not hold");
                report.unanswered = report.unanswered.saturating_add(1);
            }
            Answer::Failed(error) => {
                report.failed = report.failed.saturating_add(1);
                warn!(case = %case.id, %error, "review request failed");
            }
            Answer::Answered {
                verdicts: triaged,
                dropped,
            } => {
                report.answered = report.answered.saturating_add(1);
                report.dropped = report.dropped.saturating_add(dropped);
                let (rows, states, tally) =
                    record_case(case, triaged, reviewer, observed_at);
                report.absorb(tally);
                verdicts.extend(rows);
                closed.extend(states);
            }
        }
    }
    Ok((verdicts, closed))
}

pub(crate) fn compute_digest(
    verdicts: &[census_domain::model::ReviewVerdictRecord],
    cases: &[census_domain::model::ReviewCase],
) -> StoreResult<String> {
    census_domain::model::serialized_digest(&(verdicts, cases)).map_err(|source| {
        census_store::StoreError::Json {
            detail: "cannot hash review checkpoint".to_owned(),
            source,
        }
    })
}

fn commit_checkpoint(
    store: &Store,
    verdicts: &[census_domain::model::ReviewVerdictRecord],
    cases: &[ReviewCase],
    observed_at: &str,
    checkpoint: usize,
) -> StoreResult<Application> {
    let mut batch = store.write_batch();
    let digest = compute_digest(verdicts, cases)?;
    let operation = format!("review:{observed_at}:{checkpoint}:{digest}");
    batch.replace_many(Table::IdentityVerdicts, verdicts)?;
    batch.replace_many(Table::ReviewCases, cases)?;
    batch.commit_once(&operation, &digest)
}

fn should_write(dry_run: bool, minted: usize) -> bool {
    !dry_run && minted > 0
}

async fn process_checkpoint(
    store: &Store,
    chunk: &[(ReviewCase, ReviewFamily)],
    clients: &[ModelClient],
    options: &ReviewOptions,
    observed_at: &str,
    checkpoint: usize,
    report: &mut ReviewReport,
) -> StoreResult<()> {
    let subjects = SubjectIndex::read(store, chunk)?;
    let asked = ask_lanes(chunk, &subjects, clients).await;
    let (verdicts, closed) = process_answers(chunk, asked, observed_at, report)?;

    if !should_write(options.dry_run, verdicts.len()) {
        return Ok(());
    }
    commit_checkpoint(store, &verdicts, &closed, observed_at, checkpoint)?;
    Ok(())
}

pub async fn run_lanes(
    store: &Store,
    clients: &[ModelClient],
    options: &ReviewOptions,
    observed_at: &str,
) -> StoreResult<ReviewReport> {
    let mut report = ReviewReport::default();
    let pending = pending_cases(store, options)?;
    report.requested = pending.len();
    if pending.is_empty() || clients.is_empty() {
        return Ok(report);
    }

    let chunks = pending.chunks(CHECKPOINT_CASES);
    for (chunk_idx, chunk) in chunks.enumerate() {
        process_checkpoint(
            store,
            chunk,
            clients,
            options,
            observed_at,
            chunk_idx,
            &mut report,
        )
        .await?;
    }
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
