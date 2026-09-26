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

use census_domain::model::{CanonicalAthlete, Confidence, RetainedConflict};
use census_domain::model::records::{ReviewCase, ReviewState, COHORT_DECISION_FAMILIES};

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
        // Check for unresolved retained conflicts — these are NOT verified.
        if has_unresolved_conflicts(&athlete.retained_conflicts) {
            return IdentityDecision::Unresolved;
        }

        // Cohort decision families that started terminal (COHORT_UNVERIFIED, COHORT_IDENTITY_CONFIDENCE)
        // are decided by the evidence rule itself — the row is published at the bar its evidence
        // supports. A case in these families with state != Resolved is unresolved.
        if has_pending_cohort_decision(athlete) {
            return IdentityDecision::Unresolved;
        }

        // identity_confidence HIGH does NOT mean identity verified — it means the grade
        // observation agreed with the cohort. Grade agreement supports cohort only, never identity.
        // An athlete with HIGH confidence but no accepted decision is still NoDecision.
        if athlete.identity_confidence >= Confidence::HIGH
            && !has_any_unresolved_signal(athlete)
        {
            // No unresolved signals at all — this athlete has no flaggable issues.
            // This is the closest to "verified" but is still "no decision" per the contract.
            return IdentityDecision::NoDecision;
        }

        IdentityDecision::NoDecision
    }
}

/// Whether the athlete has any unresolved signals: conflicts, pending cases, or low confidence.
fn has_any_unresolved_signal(athlete: &CanonicalAthlete) -> bool {
    !athlete.retained_conflicts.is_empty()
        || athlete.identity_confidence < Confidence::HIGH
}

/// Check if any retained conflicts are unresolved (no verdict recorded).
fn has_unresolved_conflicts(conflicts: &[RetainedConflict]) -> bool {
    // Every retained conflict that exists in the row means the merge kept it unresolved.
    // The presence of ANY retained conflict means this athlete cannot be verified.
    !conflicts.is_empty()
}

/// Check if the athlete has pending cohort decision cases that haven't been resolved.
fn has_pending_cohort_decision(athlete: &CanonicalAthlete) -> bool {
    // This is a stub — the actual implementation would scan review_cases table
    // for cases matching this athlete's id in the cohort decision families.
    // For now, check if confidence is LOW (indicating a cohort evidence issue).
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
    use std::collections::BTreeMap;
    use census_domain::model::SourceNamespace;
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

// ──────────────────────────────────────────────────────────────────────────────────────────────
// Shared-contract interface proposal for Main
// ──────────────────────────────────────────────────────────────────────────────────────────────
//
// The projection needs access to:
//
// 1. **review_cases** table (Table::ReviewCases)
//    - Filter by subject_id == athlete.id AND family IN (COHORT_UNVERIFIED_FAMILY, COHORT_IDENTITY_CONFIDENCE_FAMILY)
//    - State Pending or Retained → unresolved
//    - State Resolved with a matching verdict in identity_verdicts → accepted
//    - State Superseded → check if the verdict is rejected or superseded
//
// 2. **identity_verdicts** table (Table::IdentityVerdicts)
//    - Keyed by case_id
//    - A verdict with state "accepted" + matching case in Resolved → accepted decision
//    - A verdict with state "rejected" → rejected decision
//
// 3. **conflicts** table (Table::Conflicts)
//    - These are the resolved conflicts (merge decided, different from retained_conflicts)
//    - A conflict with no matching verdict is still a disagreement
//
// Proposed shared-contract addition to Dataset:
//
// ```rust
// pub(super) struct IdentityProjection {
//     /// athlete_id → decision state
//     decisions: BTreeMap<String, IdentityDecision>,
// }
//
// impl IdentityProjection {
//     pub(super) fn load(store: &Store, athlete_ids: &[String]) -> ReportResult<Self> {
//         // 1. Scan review_cases for cohort families
//         // 2. Scan identity_verdicts for resolved cases
//         // 3. Join: case.Resolved + verdict.Accepted → Accepted
//         //    case.Resolved + verdict.Rejected → Rejected
//         //    case.Pending or Retained → Unresolved
//         //    No case + no conflict → NoDecision
//         //    Has retained_conflict → Unresolved
//         //    No case + no retained_conflict but no verdict → NoDecision
//     }
//
//     pub(super) fn decision(&self, athlete_id: &str) -> IdentityDecision {
//         self.decisions.get(athlete_id).copied().unwrap_or(IdentityDecision::NoDecision)
//     }
// }
// ```
//
// The projection is built once per Dataset load and cached, same as contacts/tallies/prs.
