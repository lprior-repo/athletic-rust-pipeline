//! The review lane: ask a local model about the findings the merge retained, and keep only answers
//! the store's own evidence can back.
//!
//! The retained families are findings with an unresolved *field* — a school no source placed in a
//! jurisdiction, a meet whose venue nobody named, two athlete rows the merge kept apart under one
//! key. Each is a question a small local model can answer from the row's own text, and each answer is
//! checkable: this module asks, validates, and records.
//!
//! Three rules shape the code:
//!
//! * **The lane asks about what the store retained.** Packets are built from `ReviewCase` rows and
//!   the subject's own canonical fields; the model never sees a question this store did not ask.
//! * **Validation is local.** A proposal is admitted only when it satisfies the family's own rule —
//!   see [`validate`] — so a model can be wrong without being able to move a row.
//! * **A verdict is evidence, not an edit.** The pass writes the verdict and moves the case out of
//!   `pending`; it never rewrites a canonical row. Canonical data belongs to the merge, which is the
//!   only component that owns those tables.
//!
//! Layout: `families` states what a pass asks about and how it is configured, `packets` builds each
//! question out of the store, `athlete_packet` builds the one question that compares two rows,
//! `athlete_verdict` holds that family's answers, `ask` makes the request, `verdicts` validates what
//! came back, `records` turns it into durable rows and the report, and [`run`] drives one pass.

mod ask;
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
use tracing::{info, warn};

use census_store::{Store, StoreResult, Table};

use ask::{ask_case, Answer};
use packets::{pending_cases, SubjectIndex};
use records::record_case;

pub use athlete_verdict::AthleteVerdict;
pub use families::{ReviewFamily, ReviewOptions};
pub use model::{ModelClient, ModelError, ModelOptions};
pub use records::ReviewReport;
pub use verdicts::{triage, validate, Adjudication, Admitted, Refusal};

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

/// Ask every pending case, keeping exactly one request in flight per lane.
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

/// Process answers and record verdicts for a batch of reviewed cases.
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

/// Run one pass across several model lanes.
///
/// Each lane is an OpenAI-compatible server; the local llama.cpp lanes run a single slot
/// (`-np 1`), so the pass keeps exactly one request in flight per lane and cycles cases across
/// them. Answers are consumed in the order the cases were asked, so a pass stays deterministic
/// no matter how many lanes answered it, and the durable writes still happen once, in order,
/// after every answer is in hand.
pub async fn run_lanes(
    store: &Store,
    clients: &[ModelClient],
    options: &ReviewOptions,
    observed_at: &str,
) -> StoreResult<ReviewReport> {
    let mut report = ReviewReport::default();
    let pending = pending_cases(store, options)?;
    report.asked = pending.len();
    if pending.is_empty() || clients.is_empty() {
        return Ok(report);
    }

    let subjects = SubjectIndex::read(store, &pending)?;
    let asked = ask_lanes(&pending, &subjects, clients).await;
    let (verdicts, closed) = process_answers(&pending, asked, observed_at, &mut report)?;

    if should_write(options.dry_run, verdicts.len()) {
        store.replace_many(Table::IdentityVerdicts, &verdicts)?;
        store.replace_many(Table::ReviewCases, &closed)?;
    }
    info!(summary = %report.summary(), "review pass finished");
    Ok(report)
}

/// Whether a pass writes its verdicts: a dry run never writes, and a pass that minted none has
/// nothing to write.
fn should_write(dry_run: bool, minted: usize) -> bool {
    !dry_run && minted > 0
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "athlete_tests.rs"]
mod athlete_tests;
