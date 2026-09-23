//! What the two queues say: a finding keys its row, and evidence that changed mints a new case
//! instead of borrowing the answer given about the old one.

use super::super::*;

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
fn evidence_normalization_ignores_order_case_and_whitespace() {
    let ordered = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith",
        "ids: ath_a, ath_b",
    );
    let reordered = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "jordan smith",
        "ids:  ATH_B,   ath_a ",
    );
    assert_eq!(
        ordered.id, reordered.id,
        "the same facts in another order, case or spacing are the same evidence"
    );

    let changed = ReviewCase::pending(
        "Athlete identity",
        "ath_a",
        "Jordan Smith",
        "ids: ath_a, ath_c",
    );
    assert_ne!(ordered.id, changed.id, "one changed fact is new evidence");
}
