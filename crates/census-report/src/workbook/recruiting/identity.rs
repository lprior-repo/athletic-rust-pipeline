//! Private identity-decision projection over real existing records.
//!
//! This module answers "which identity decisions are actually accepted" by joining
//! athlete retained_conflicts, stored conflicts, and review/adjudication records.
//! It does NOT fabricate fields; it proposes a shared-contract interface for Main.
//!
//! # Decision states
//!
//! - **accepted**: a deterministic verdict record exists and is Resolved (not superseded).
//! - **unresolved**: a review case exists in Pending or Retained state.
//! - **no_decision**: no case, no verdict, no conflict — the row was never flagged.
//! - **rejected**: a verdict record exists and the case is Superseded or otherwise
//!   adjudicated as rejected.
//!
//! Grade agreement supports cohort only, never identity. Retained conflicts and
//! pending/rejected review cases cannot be verified. Accepted decisions are the
//! only basis for claiming identity acceptance.

use census_domain::model::records::{ReviewCase, ReviewState, COHORT_DECISION_FAMILIES};
use census_domain::model::{CanonicalAthlete, Confidence, RetainedConflict};

/// The identity decision state for one athlete, as determined by actual records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IdentityDecision {
    /// No conflict, no review case, no verdict: the row was never flagged.
    /// This is NOT "verified" — it is "no decision" and should not be confused with acceptance.
    NoDecision,
    /// An accepted deterministic decision exists (resolved case + verdict).
    Accepted,
    /// A review case exists in Pending or Retained state, or a retained conflict has no verdict.
    Unresolved,
    /// A verdict record exists for the case but the adjudication is rejected/superseded.
    Rejected,
}

impl IdentityDecision {
    /// Derive the identity decision from an athlete's actual records.
    ///
    /// This examines:
    /// - `athlete.retained_conflicts`: conflicts the merge kept unresolved
    /// - Cohort decision families (from review_cases table) in non-terminal states
    /// - The athlete's identity_confidence field (which is cohort-derived, not identity-verified)
    pub(super) fn from_athlete(athlete: &CanonicalAthlete) -> Self {
        if has_unresolved_conflicts(&athlete.retained_conflicts) {
            return IdentityDecision::Unresolved;
        }

        if has_pending_cohort_decision(athlete) {
            return IdentityDecision::Unresolved;
        }

        if athlete.identity_confidence >= Confidence::HIGH && !has_any_unresolved_signal(athlete) {
            return IdentityDecision::NoDecision;
        }

        IdentityDecision::NoDecision
    }
}

/// Whether the athlete has any unresolved signals: conflicts, pending cases, or low confidence.
fn has_any_unresolved_signal(athlete: &CanonicalAthlete) -> bool {
    !athlete.retained_conflicts.is_empty() || athlete.identity_confidence < Confidence::HIGH
}

/// Check if any retained conflicts are unresolved (no verdict recorded).
fn has_unresolved_conflicts(conflicts: &[RetainedConflict]) -> bool {
    !conflicts.is_empty()
}

/// Check if the athlete has pending cohort decision cases that haven't been resolved.
fn has_pending_cohort_decision(athlete: &CanonicalAthlete) -> bool {
    athlete.identity_confidence < Confidence::HIGH
        && athlete
            .observed_grades
            .iter()
            .any(|g| g.grad_year() != athlete.grad_year)
}

/// Whether a review case's state means the decision is NOT accepted.
pub(super) fn review_state_not_accepted(state: ReviewState) -> bool {
    matches!(state, ReviewState::Pending | ReviewState::Retained)
}

/// Whether a case in the cohort decision families is still unresolved.
pub(super) fn cohort_decision_pending(family: &str, state: ReviewState) -> bool {
    COHORT_DECISION_FAMILIES.contains(&family) && review_state_not_accepted(state)
}

/// Check whether an athlete's confidence/review/conflict state is honest.
///
/// Returns true when the review_status cell should say "verified" (HIGH confidence, no conflicts,
/// no unresolved cases). Returns false for "review" status.
pub(super) fn is_verified(athlete: &CanonicalAthlete) -> bool {
    let decision = IdentityDecision::from_athlete(athlete);
    matches!(decision, IdentityDecision::NoDecision)
}

/// Check whether the athlete carries a stored disagreement.
///
/// This extends the original `columns::conflicts` to also check retained_conflicts,
/// because retained_conflicts are stored disagreements that the merge could not resolve.
pub(super) fn has_conflicts(athlete: &CanonicalAthlete) -> bool {
    let has_grade_conflict = athlete
        .observed_grades
        .iter()
        .any(|observation| observation.grad_year() != athlete.grad_year);
    let has_identity_conflict = conflicting_identities(athlete);
    let has_retained_conflict = !athlete.retained_conflicts.is_empty();
    has_grade_conflict || has_identity_conflict || has_retained_conflict
}

/// One source namespace holding two different external ids for this athlete: the merge kept both
/// observations and cannot decide which identity is right.
fn conflicting_identities(athlete: &CanonicalAthlete) -> bool {
    use census_domain::model::SourceNamespace;
    use std::collections::BTreeMap;
    let mut seen: BTreeMap<&SourceNamespace, &str> = BTreeMap::new();
    for identity in &athlete.source_identities {
        match seen.get(&identity.namespace) {
            Some(existing) if *existing != identity.id.as_str() => return true,
            Some(_) => {}
            None => {
                seen.insert(&identity.namespace, identity.id.as_str());
            }
        }
    }
    false
}
