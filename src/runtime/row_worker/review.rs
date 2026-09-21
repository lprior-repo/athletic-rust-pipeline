use super::support::publish_review_input;
use crate::{
    domain::{
        decision::{self, CandidateReason},
        evidence::ProfileEvidence,
    },
    model::SourceRecord,
    runtime::{
        protocol::{ReviewCandidate, ReviewInput, ReviewOutcome, ReviewVerdict},
        review_case::{case_key, ReviewCaseClient},
        row_protocol::{AcceptanceMethod, RowResolution},
        Runtime,
    },
};
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(crate) async fn resolve_assessment(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    source: &SourceRecord,
    profiles: &[ProfileEvidence],
    assessment: &(
        crate::domain::identity::EvidenceDigest,
        decision::Assessment,
    ),
) -> std::result::Result<(RowResolution, Option<ReviewOutcome>, Vec<String>), TerminalError> {
    let value = &assessment.1;
    if value.is_deterministic_acceptance() {
        return Ok(deterministic_resolution(value));
    }
    if matches!(value.decision(), decision::Decision::CompleteSearchNoMatch) {
        return Ok((RowResolution::CompleteSearchNoMatch, None, Vec::new()));
    }
    let eligible = value
        .candidates()
        .iter()
        .filter(|candidate| candidate.hard_eligible())
        .collect::<Vec<_>>();
    if value.decision() != decision::Decision::IdentityReview || !(2..=64).contains(&eligible.len())
    {
        return Ok(review_required(
            "assessment requires review without a valid 2..=64 hard-eligible candidate set",
        ));
    }
    let Some(candidates) = review_candidates(profiles, &eligible) else {
        return Ok(review_required(
            "one or more hard-eligible candidates lacked a deserialized profile",
        ));
    };
    let input = ReviewInput {
        source: source.clone(),
        candidates,
    };
    let digest = match publish_review_input(ctx, runtime.clone(), input.clone()).await {
        Ok(value) => value,
        Err(error) if error.code() == 409 => return Err(error),
        Err(error) => {
            return Ok(review_required(format!(
                "review input exceeds 64 KiB or could not be serialized: {error}"
            )))
        }
    };
    let key = match review_case_key(runtime.clone(), input).await {
        Ok(key) => key,
        Err(issue) => return Ok(issue),
    };
    let outcome = match review_round_trip(ctx, &key, digest).await? {
        Ok(outcome) => outcome,
        Err(issue) => return Ok(review_required(issue)),
    };
    Ok(applied_resolution(value, outcome))
}

/// Maps a deterministic acceptance onto its row resolution without consulting the reviewer.
fn deterministic_resolution(
    value: &decision::Assessment,
) -> (RowResolution, Option<ReviewOutcome>, Vec<String>) {
    (
        value
            .accepted_athlete_id()
            .map_or(RowResolution::ReviewRequired, |athlete_id| {
                RowResolution::Accepted {
                    athlete_id,
                    method: AcceptanceMethod::Deterministic,
                }
            }),
        None,
        Vec::new(),
    )
}

/// A row that requires review, carrying the single issue that explains why.
fn review_required(
    issue: impl Into<String>,
) -> (RowResolution, Option<ReviewOutcome>, Vec<String>) {
    (RowResolution::ReviewRequired, None, vec![issue.into()])
}

/// Pairs each hard-eligible candidate with its deserialized profile. `None` means at least one
/// candidate had no profile in `profiles`.
fn review_candidates(
    profiles: &[ProfileEvidence],
    eligible: &[&CandidateReason],
) -> Option<Vec<ReviewCandidate>> {
    let candidates = eligible
        .iter()
        .filter_map(|candidate| {
            profiles
                .iter()
                .find(|profile| profile.athlete_id == candidate.athlete_id())
                .map(|profile| review_candidate(profile, candidate))
        })
        .collect::<Vec<_>>();
    (candidates.len() == eligible.len()).then_some(candidates)
}

/// Computes the review case key. A key failure is a row-level review requirement, not an abort.
async fn review_case_key(
    runtime: Arc<Runtime>,
    input: ReviewInput,
) -> std::result::Result<String, (RowResolution, Option<ReviewOutcome>, Vec<String>)> {
    let config = runtime.config.clone();
    match runtime.blocking(move || case_key(&config, &input)).await {
        Ok(value) => Ok(value),
        Err(error) => Err(review_required(format!("review case key failed: {error}"))),
    }
}

/// Drives one review case invocation. `Err` is the cancelled-lane signal; a failed call is reported
/// as an issue the caller turns into a review requirement.
async fn review_round_trip(
    ctx: &ObjectContext<'_>,
    key: &str,
    digest: crate::domain::identity::EvidenceDigest,
) -> std::result::Result<std::result::Result<ReviewOutcome, String>, TerminalError> {
    let call = ctx
        .object_client::<ReviewCaseClient>(key)
        .review(Json(digest))
        .call();
    let handle = call.invocation_handle().await?;
    match call.await {
        Ok(value) => Ok(Ok(value.0)),
        Err(error) if error.code() == 409 => {
            handle.cancel();
            Err(error)
        }
        Err(error) => Ok(Err(format!("review worker call failed: {error}"))),
    }
}

/// Applies the reviewer's verdict to the assessment and reports the resulting row resolution.
fn applied_resolution(
    value: &decision::Assessment,
    outcome: ReviewOutcome,
) -> (RowResolution, Option<ReviewOutcome>, Vec<String>) {
    let choice = match &outcome {
        ReviewOutcome::Reviewed {
            verdict: ReviewVerdict::Select { athlete_id, .. },
            ..
        } => decision::ReviewChoice::Select(*athlete_id),
        ReviewOutcome::Reviewed {
            verdict: ReviewVerdict::Unresolved { .. },
            ..
        }
        | ReviewOutcome::Failed { .. } => decision::ReviewChoice::Unresolved,
    };
    match decision::apply_review(value, choice) {
        Ok(final_decision) => match final_decision.accepted_athlete_id() {
            Some(athlete_id) => (
                RowResolution::Accepted {
                    athlete_id,
                    method: AcceptanceMethod::LocalReview,
                },
                Some(outcome),
                Vec::new(),
            ),
            None => (
                RowResolution::ReviewRequired,
                Some(outcome),
                vec!["review did not establish a valid selection".to_owned()],
            ),
        },
        Err(error) => (
            RowResolution::ReviewRequired,
            Some(outcome),
            vec![format!(
                "review selection rejected by domain gates: {error}"
            )],
        ),
    }
}

fn review_candidate(profile: &ProfileEvidence, reason: &CandidateReason) -> ReviewCandidate {
    let mut reasons = profile
        .graduation_years
        .iter()
        .map(|year| format!("observed graduation year {}", year.value.get()))
        .collect::<Vec<_>>();
    reasons.push(if profile.teams.is_empty() {
        "no team history was observed".to_owned()
    } else {
        "team history was observed".to_owned()
    });
    reasons.extend(reason.reasons().iter().cloned());
    reasons.extend(
        profile
            .issues
            .iter()
            .map(|issue| format!("evidence issue: {}", issue.code)),
    );
    ReviewCandidate {
        athlete_id: profile.athlete_id,
        name: profile.name.value.clone(),
        teams: profile.teams.clone(),
        graduation_years: profile.graduation_years.clone(),
        sports: profile.sports.clone(),
        issues: profile.issues.clone(),
        eligibility_reasons: reasons,
        documents: profile.documents.clone(),
    }
}
