//! Human review transitions: what a supplied review choice may override.

use super::{
    Assessment, CandidateReason, Decision, FinalDecision, ReviewChoice, SearchCompleteness,
};
use crate::domain::identity::AthleteId;
use anyhow::{bail, Result};

pub fn apply_review(assessment: &Assessment, choice: ReviewChoice) -> Result<FinalDecision> {
    match choice {
        ReviewChoice::Unresolved => Ok(assessment
            .accepted_athlete_id()
            .map_or(FinalDecision { accepted: None }, FinalDecision::accepted)),
        ReviewChoice::Select(athlete_id) => apply_selection(assessment, athlete_id),
    }
}

fn apply_selection(assessment: &Assessment, athlete_id: AthleteId) -> Result<FinalDecision> {
    if !matches!(assessment.search, SearchCompleteness::Complete { .. }) {
        bail!("selection cannot override incomplete discovery");
    }
    if matches!(
        assessment.decision,
        Decision::EvidenceReview | Decision::CompleteSearchNoMatch
    ) {
        bail!("selection cannot override insufficient or contradictory evidence")
    }
    let candidate = assessment
        .candidates
        .iter()
        .find(|candidate| candidate.athlete_id == athlete_id && candidate.hard_eligible)
        .ok_or_else(|| anyhow::anyhow!("selected athlete is not an eligible supplied candidate"))?;
    if matches!(assessment.decision, Decision::IdentityReview)
        && !selection_distinguishes(assessment, candidate)
    {
        bail!("supplied evidence does not distinguish the selected candidate")
    }
    Ok(FinalDecision::accepted(athlete_id))
}

fn selection_distinguishes(assessment: &Assessment, selected: &CandidateReason) -> bool {
    assessment
        .candidates
        .iter()
        .filter(|candidate| candidate.hard_eligible)
        .filter(|candidate| {
            candidate.exact_name == selected.exact_name
                && candidate.matching_school == selected.matching_school
                && candidate.location_corroborated == selected.location_corroborated
                && candidate.mailing_location_matches == selected.mailing_location_matches
        })
        .count()
        == 1
}
