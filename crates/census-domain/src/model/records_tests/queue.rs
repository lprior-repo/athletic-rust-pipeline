//! What the two queues say: a finding keys its row, and evidence that changed mints a new case
//! instead of borrowing the answer given about the old one.

use super::super::review_record::REVIEW_POLICY_REVISION;
use super::super::*;
use crate::model::AthleteCandidateId;

#[test]
fn conflict_and_case_ids_bind_the_finding_not_the_run() {
    let first = RetainedConflict::new("School identity", "sch:a", "A (WI)", "shared name");
    let second = RetainedConflict::new("School identity", "sch:a", "A (WI)", "shared name");
    assert_eq!(first.id, second.id);
    assert_eq!(first.id, "School identity:sch:a");

    let case = ReviewCase::pending(
        "Meet venue unresolved",
        "meet:m1",
        "Invite (2026-04-01)",
        "no venue",
    );
    assert_eq!(case.state, ReviewState::Pending);
    assert!(
        case.id.starts_with(&format!(
            "Meet venue unresolved:meet:m1:p{REVIEW_POLICY_REVISION}:"
        )),
        "the id binds the finding, the policy revision and the evidence digest: {}",
        case.id
    );
    assert_eq!(
        case,
        ReviewCase::pending(
            "Meet venue unresolved",
            "meet:m1",
            "Invite (2026-04-01)",
            "no venue"
        ),
        "one derivation of one finding is one case"
    );
    assert_eq!(ReviewState::default(), ReviewState::Pending);
}

#[test]
fn new_evidence_mints_a_new_case_and_leaves_the_decided_one_as_history() {
    let detail = "same school, name and cohort as every id here: ath_a, ath_b";
    let first = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith (Madison West High School)",
        detail,
    );
    let mut decided = first.clone();
    decided.state = ReviewState::Resolved;

    let again = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith (Madison West High School)",
        detail,
    );
    assert_eq!(
        again.id, first.id,
        "the same evidence must reuse the case the operator already answered"
    );

    let grown = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith (Madison West High School)",
        "same school, name and cohort as every id here: ath_a, ath_b, ath_c",
    );
    assert_ne!(
        grown.id, first.id,
        "a third row joined the group: that is evidence the decided case never saw"
    );
    assert_eq!(grown.state, ReviewState::Pending);
    assert_eq!(
        decided.state,
        ReviewState::Resolved,
        "the earlier case, and the verdict recorded under its id, stay readable as history"
    );
}

#[test]
fn evidence_normalization_ignores_case_and_whitespace() {
    let written = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith",
        "same school and class as the row, ids: ath_a, ath_b",
    );
    let respaced = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "  jordan   smith ",
        "same school and class as the row, IDS:  ath_a,   ath_b ",
    );
    assert_eq!(
        written.id, respaced.id,
        "the same statement cased or spaced another way is the same evidence"
    );

    let changed = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith",
        "same school and class as the row, ids: ath_a, ath_c",
    );
    assert_ne!(written.id, changed.id, "one changed fact is new evidence");
}

#[test]
fn an_attribution_swap_is_not_the_same_evidence() {
    // The same tokens with the opposite meaning: only the association between each side and its value
    // differs. Folding a statement into a sorted bag of tokens minted one case for both, so a verdict
    // taken about one package of evidence was read as a verdict about the other.
    let forward = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith",
        "side_a grad_year 2027 side_b grad_year 2028",
    );
    let swapped = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith",
        "side_b grad_year 2028 side_a grad_year 2027",
    );
    assert_ne!(
        forward.id, swapped.id,
        "the same tokens attributed the other way round are different evidence"
    );
}

#[test]
fn a_sign_and_a_unit_are_part_of_the_value() {
    // Surrounding punctuation is formatting and goes; a sign is the value's own and stays, or a
    // headwind and a tailwind would read as one measurement.
    let tailwind = ReviewCase::pending("Athlete identity", "ath_a", "Row", "wind_mps +1.4");
    let headwind = ReviewCase::pending("Athlete identity", "ath_a", "Row", "wind_mps -1.4");
    assert_ne!(
        tailwind.id, headwind.id,
        "-1.4 m/s is not +1.4 m/s: the sign is not punctuation"
    );

    let metres = ReviewCase::pending("Athlete identity", "ath_a", "Row", "mark 17.02m");
    let seconds = ReviewCase::pending("Athlete identity", "ath_a", "Row", "mark 17.02s");
    assert_ne!(metres.id, seconds.id, "the unit is part of the value");
}

#[test]
fn a_findings_membership_is_part_of_its_evidence() {
    let committee = |suffixes: &[&str]| {
        ReviewCase::pending_with_evidence(
            "Athlete identity",
            "ath_a",
            "Jordan Smith",
            "one provider object on several canonical rows",
            CaseEvidence::of([
                "Jordan Smith",
                "one provider object on several canonical rows",
            ])
            .with_members(
                MEMBER_SET_LABEL,
                suffixes
                    .iter()
                    .map(|suffix| AthleteCandidateId::mint("ath", &[*suffix])),
            ),
        )
    };

    let two = committee(&["c1", "c2"]);
    let three = committee(&["c1", "c2", "c3"]);
    assert_ne!(
        two.id, three.id,
        "a finding resting on three rows is new evidence beside the one that rested on two"
    );

    let reordered = committee(&["c2", "c1"]);
    assert_eq!(
        two.id, reordered.id,
        "the same members listed in another order are the same group, not a new question"
    );
}

/// A family the census's own rules decide is minted decided: the case is not a question anyone is
/// holding open, and the row still reaches the workbook's queues.
#[test]
fn a_rule_decided_family_is_minted_retained() {
    let cohort = ReviewCase::minted(
        COHORT_UNVERIFIED_FAMILY,
        "coach:1",
        "A Coach (Somewhere High)",
        "no grade observation was retained",
    );
    assert_eq!(cohort.state, ReviewState::Retained);
    assert_eq!(
        cohort.id,
        ReviewCase::pending(
            COHORT_UNVERIFIED_FAMILY,
            "coach:1",
            "A Coach (Somewhere High)",
            "no grade observation was retained",
        )
        .id,
        "the state is not part of the id: the evidence is what a case is keyed on"
    );

    let venue = ReviewCase::minted(
        UNRESOLVED_VENUE_FAMILY,
        "meet:1",
        "Some Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    assert_eq!(venue.state, ReviewState::Pending);
}

/// The cohort families are decided by the rule the row is read through, not by a later answer.
#[test]
fn a_cohort_claim_its_evidence_does_not_raise_is_minted_retained() {
    for family in COHORT_DECISION_FAMILIES {
        assert!(ReviewCase::decided_by_its_own_rules(family), "{family}");
        let case = ReviewCase::minted(
            family,
            "athlete:1",
            "A Runner",
            "no grade observation retained",
        );
        assert_eq!(
            case.state,
            ReviewState::Retained,
            "{family} is published at the bar its own evidence supports, so nothing is owed for it"
        );
        assert_eq!(
            case.id,
            ReviewCase::pending(
                family,
                "athlete:1",
                "A Runner",
                "no grade observation retained"
            )
            .id,
            "the state is not part of the id for these either: the evidence keys the case"
        );
    }
    // The families this store cannot answer for itself stay the lane's work.
    for family in [
        UNRESOLVED_SCHOOL_FAMILY,
        UNRESOLVED_VENUE_FAMILY,
        ATHLETE_IDENTITY_FAMILY,
    ] {
        assert!(!ReviewCase::decided_by_its_own_rules(family), "{family}");
        assert_eq!(
            ReviewCase::minted(family, "subject:1", "A Subject", "the finding").state,
            ReviewState::Pending,
            "{family} is a question only outside evidence settles"
        );
    }
}
