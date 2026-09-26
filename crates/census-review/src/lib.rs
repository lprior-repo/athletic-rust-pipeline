//! The review lane: ask a local model about the findings the merge retained, and keep only answers
//! the store's own evidence can back.
//!
//! The retained families are findings with an unresolved *field* - a school no source placed in a
//! jurisdiction, a meet whose venue nobody named, two athlete rows the merge kept apart under one
//! key. Each is a question a small local model can answer from the row's own text, and each answer is
//! checkable: this module asks, validates, and records.
//!
//! Three rules shape the code:
//!
//! * **The lane asks about what the store retained.** Packets are built from `ReviewCase` rows and
//!   the subject's own canonical fields; the model never sees a question this store did not ask.
//! * **Validation is local.** A proposal is admitted only when it satisfies the family's own rule -
//!   see [`validate`] - so a model can be wrong without being able to move a row.
//! * **A verdict is evidence, not an edit.** The pass writes the verdict and moves the case out of
//!   `pending`; it never rewrites a canonical row. Canonical data belongs to the merge, which is the
//!   only component that owns those tables.
//!
//! Layout: `families` states what a pass asks about and how it is configured, `packets` builds each
//! question out of the store, `athlete_packet` builds the one question that compares two rows,
//! `athlete_verdict` holds that family's answers, `ask` makes the request, `verdicts` validates what
//! came back, `records` turns it into durable rows and the report, and [`run`] drives one pass.
//!
//! # Checkpointed processing
//!
//! `run_lanes` processes pending cases in bounded chunks (see [`CHECKPOINT_CASES`]). Each chunk
//! stages both `IdentityVerdicts` and `ReviewCases` into one [`StoreBatch`] and commits through
//! [`StoreBatch::commit_once`], so a crash writes nothing or writes both tables atomically. The
//! operation id is deterministic (derived from `observed_at` and the chunk index), so a replay
//! of the same pass reproduces the same ids and writes nothing new - the store answers
//! [`Application::Repeated`].
//!
//! # Report stages
//!
//! The report tracks four stages this crate can observe:
//! `requested` (cases selected for asking), `answered` (answers returned, incl. failures/cancellations),
//! `decided` (real decisions via `Adjudication::Decided`), and `accepted` (rows written to the
//! verdict table). The fifth stage, `applied` - whether a verdict was applied to an athlete's
//! identity - is measured by the reporting layer, not this crate.
//!
//! A deterministic decision is not a model call. A stored verdict count is not an accuracy
//! measurement.

#![forbid(unsafe_code)]

mod ask;
mod athlete_cluster_findings;
mod athlete_clusters;
/// Public because this module holds the one definition of the athlete group key: the census crate's
/// workbook groups its conflict queue by it, so the queue and the lane cannot drift into two rules.
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
use sha2::{Digest, Sha256};
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

/// Maximum pending cases to ask per checkpoint.
///
/// Memory is bounded to this many case structs, their packets, and their answers. A checkpoint
/// writes at most `CHECKPOINT_CASES` verdicts and `CHECKPOINT_CASES` case states through one
/// `StoreBatch` / one `SyncData` commit. For the default 256: ~256 * ~1 KB = ~256 KB per
/// checkpoint, well within a single page of RAM.
///
/// A checkpoint is one durability boundary: a crash between checkpoints loses nothing (the case
/// stays pending and the next pass re-asks it); a crash mid-checkpoint is atomic because both
/// tables commit in one `fdatasync` via `commit_once`.
const CHECKPOINT_CASES: usize = 256;

/// Run one pass: ask each retained case's subject, validate the answer, record the verdict.
///
/// `observed_at` dates the pass the way an index pass's `finished_at` does.
pub async fn run(
    store: &Store,
    client: &ModelClient,
    options: &ReviewOptions,
    observed_at: &str,
) -> StoreResult<ReviewReport> {
    run_lanes(store, std::slice::from_ref(client), options, observed_at).await
}

/// Ask every pending case in this chunk, keeping exactly one request in flight per lane.
///
/// Answers come back in the order the cases were asked, so a pass stays deterministic no matter
/// how many lanes answered it. The caller guarantees a non-empty roster on both sides: `pending`
/// cases and at least one client.
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
            (index, client.options().model.as_str(), answer)
        })
        .buffered(lanes)
        .collect()
        .await
}

/// Process answers and record verdicts for a chunk of reviewed cases.
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
        let Some((case, family)) = pending.get(index) else {
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
                    record_case(case, *family, triaged, reviewer, observed_at);
                report.absorb(tally);
                verdicts.extend(rows);
                closed.extend(states);
            }
        }
    }
    Ok((verdicts, closed))
}

/// Compute a SHA-256 digest of the verdicts and cases' serialized JSON.
pub(crate) fn compute_digest(
    verdicts: &[census_domain::model::ReviewVerdictRecord],
    cases: &[census_domain::model::ReviewCase],
) -> String {
    let mut hasher = Sha256::new();
    for v in verdicts {
        hasher.update(serde_json::to_string(v).unwrap_or_default().as_bytes());
    }
    for c in cases {
        hasher.update(serde_json::to_string(c).unwrap_or_default().as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Stage both tables and commit through one receipt.
///
/// The operation id names the pass *and* its payload: `observed_at` and the checkpoint index date
/// the pass, the digest is what makes the application idempotent. Both halves are load-bearing. A
/// byte-identical replay of one checkpoint answers [`Application::Repeated`] and writes nothing,
/// however many times the pass is resumed. A later pass on the same day that decides *new* cases
/// carries a different payload, and it must be allowed through — a `review` run follows every
/// collection cycle, and a day holds more than one.
///
/// So the id must not stop at the pass: an id that names only `(observed_at, checkpoint)` records
/// the first payload of the day as the operation's payload, and refuses every later, legitimate
/// one with [`StoreError::Refused`]. The collision it would detect is not a collision here, because
/// a derivation's payload is *expected* to change as the store grows.
fn commit_checkpoint(
    store: &Store,
    verdicts: &[census_domain::model::ReviewVerdictRecord],
    cases: &[ReviewCase],
    observed_at: &str,
    checkpoint: usize,
) -> StoreResult<Application> {
    let mut batch = store.write_batch();
    let digest = compute_digest(verdicts, cases);
    let operation = format!("review:{observed_at}:{checkpoint}:{digest}");
    batch.replace_many(Table::IdentityVerdicts, verdicts)?;
    batch.replace_many(Table::ReviewCases, cases)?;
    batch.commit_once(&operation, &digest)
}

/// Whether to skip writing (dry run or no verdicts minted).
fn should_write(dry_run: bool, minted: usize) -> bool {
    !dry_run && minted > 0
}

/// Process one chunk: ask lanes, process answers, commit both tables atomically.
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

/// Run one pass across several model lanes, processing cases in bounded checkpoints.
///
/// Each lane is an OpenAI-compatible server; the local llama.cpp lanes run a single slot
/// (`-np 1`), so the pass keeps exactly one request in flight per lane and cycles cases across
/// them. Answers are consumed in the order the cases were asked, so a pass stays deterministic
/// no matter how many lanes answered it.
///
/// Cases are processed in chunks of at most [`CHECKPOINT_CASES`]. Each chunk stages both
/// `IdentityVerdicts` and `ReviewCases` into one [`StoreBatch`] and commits through
/// [`StoreBatch::commit_once`], so a crash writes nothing or writes both tables atomically.
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
