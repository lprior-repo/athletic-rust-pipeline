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
) -> (RowResolution, Option<ReviewOutcome>, Vec<String>) {
    let value = &assessment.1;
    if value.is_deterministic_acceptance() {
        return (
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
        );
    }
    if matches!(value.decision(), decision::Decision::CompleteSearchNoMatch) {
        return (RowResolution::CompleteSearchNoMatch, None, Vec::new());
    }
    let eligible = value
        .candidates()
        .iter()
        .filter(|candidate| candidate.hard_eligible())
        .collect::<Vec<_>>();
    if value.decision() != decision::Decision::IdentityReview || !(2..=64).contains(&eligible.len())
    {
        return (
            RowResolution::ReviewRequired,
            None,
            vec![
                "assessment requires review without a valid 2..=64 hard-eligible candidate set"
                    .to_owned(),
            ],
        );
    }
    let candidates = eligible
        .iter()
        .filter_map(|candidate| {
            profiles
                .iter()
                .find(|profile| profile.athlete_id == candidate.athlete_id())
                .map(|profile| review_candidate(profile, candidate))
        })
        .collect::<Vec<_>>();
    if candidates.len() != eligible.len() {
        return (
            RowResolution::ReviewRequired,
            None,
            vec!["one or more hard-eligible candidates lacked a deserialized profile".to_owned()],
        );
    }
    let input = ReviewInput {
        source: source.clone(),
        candidates,
    };
    let digest = match publish_review_input(ctx, runtime.clone(), input.clone()).await {
        Ok(value) => value,
        Err(error) => {
            return (
                RowResolution::ReviewRequired,
                None,
                vec![format!(
                    "review input exceeds 64 KiB or could not be serialized: {error}"
                )],
            )
        }
    };
    let config = runtime.config.clone();
    let key_input = input;
    let key = match runtime
        .blocking(move || case_key(&config, &key_input))
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return (
                RowResolution::ReviewRequired,
                None,
                vec![format!("review case key failed: {error}")],
            )
        }
    };
    let outcome = match ctx
        .object_client::<ReviewCaseClient>(&key)
        .review(Json(digest))
        .call()
        .await
    {
        Ok(value) => value.0,
        Err(error) => {
            return (
                RowResolution::ReviewRequired,
                None,
                vec![format!("review worker call failed: {error}")],
            )
        }
    };
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
