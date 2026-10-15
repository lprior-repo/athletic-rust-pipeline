use super::*;
use crate::model::AthleteCandidateId;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn subject(label: &str) -> AthleteCandidateId {
    AthleteCandidateId::mint("ath", &[label])
}

fn case(id: &str, label: &str, state: ReviewState) -> ReviewCase {
    ReviewCase {
        id: id.into(),
        family: ATHLETE_IDENTITY_FAMILY.into(),
        subject_id: subject(label).to_string(),
        subject: label.into(),
        detail: format!("detail:{id}"),
        state,
        member_ids: Vec::new(),
    }
}

fn resolved(id: &str, labels: &[&str]) -> ReviewCase {
    let mut value = case(id, "other-subject", ReviewState::Resolved);
    value.member_ids = labels.iter().map(|label| subject(label)).collect();
    value
}

fn verdict(id: &str, accepted: bool, kind: &str) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: format!("different-record-id:{id}"),
        case_id: id.into(),
        subject_id: "different-subject".into(),
        family: "different-family".into(),
        kind: kind.into(),
        field: "different-field".into(),
        value: "different-value".into(),
        accepted,
        confidence: 0,
        rationale: "retained payload".into(),
        reviewer: "fixture".into(),
        observed_at: "2026-10-14".into(),
        member_ids: Vec::new(),
    }
}

#[test]
fn empty_inputs_preserve_empty_partitions() -> TestResult {
    check!(eq; separate_generic_identity_cases(Vec::new(), &[]), (Vec::new(), Vec::new()));
    Ok(())
}

#[test]
fn all_resolved_identity_case_members_contribute_before_partitioning() -> TestResult {
    let a = case("generic-a", "a", ReviewState::Pending);
    let b = case("generic-b", "b", ReviewState::Retained);
    let c = case("generic-c", "c", ReviewState::Pending);
    let first = resolved("specific-first", &["a", "b"]);
    let second = resolved("specific-second", &["b", "c"]);
    let input = vec![
        a.clone(),
        first.clone(),
        c.clone(),
        b.clone(),
        second.clone(),
    ];
    let (generic, bound) = separate_generic_identity_cases(input, &[]);
    check!(eq; generic, vec![a, c, b]);
    check!(eq; bound, vec![first, second]);
    Ok(())
}

#[test]
fn omitted_resolved_cases_cannot_supply_membership_from_other_sources() -> TestResult {
    let generic = case("generic", "a", ReviewState::Pending);
    let specific = resolved("specific", &["a", "b"]);
    let answer = verdict("specific", true, "value_proposed");
    let (without, held) = separate_generic_identity_cases(vec![generic.clone()], &[answer.clone()]);
    check!(without.is_empty());
    check!(eq; held, vec![generic.clone()]);
    let (with, held) =
        separate_generic_identity_cases(vec![generic.clone(), specific.clone()], &[answer]);
    check!(eq; with, vec![generic]);
    check!(eq; held, vec![specific]);
    Ok(())
}

#[test]
fn only_resolved_identity_cases_supply_the_complete_member_union() -> TestResult {
    let generic = case("generic", "a", ReviewState::Pending);
    for state in [
        ReviewState::Pending,
        ReviewState::Retained,
        ReviewState::Superseded,
    ] {
        let mut specific = resolved("specific", &["a"]);
        specific.state = state;
        let input = vec![generic.clone(), specific];
        check!(eq; separate_generic_identity_cases(input.clone(), &[]), (Vec::new(), input));
    }
    let mut other = resolved("other-family", &["a"]);
    other.family = "school_identity".into();
    let input = vec![generic, other];
    check!(eq; separate_generic_identity_cases(input.clone(), &[]), (Vec::new(), input));
    Ok(())
}

#[test]
fn resolved_subject_text_without_member_ids_does_not_supply_membership() -> TestResult {
    let input = vec![
        case("generic", "a", ReviewState::Pending),
        case("resolved-empty", "a", ReviewState::Resolved),
    ];
    check!(eq; separate_generic_identity_cases(input.clone(), &[]), (Vec::new(), input));
    Ok(())
}

#[test]
fn generic_rows_require_identity_family_empty_members_and_pending_or_retained_state() -> TestResult
{
    let specific = resolved("specific", &["a"]);
    for state in [
        ReviewState::Pending,
        ReviewState::Retained,
        ReviewState::Resolved,
        ReviewState::Superseded,
    ] {
        let row = case("generic", "a", state);
        let actual = separate_generic_identity_cases(vec![row.clone(), specific.clone()], &[]);
        let expected = match state {
            ReviewState::Pending | ReviewState::Retained => (vec![row], vec![specific.clone()]),
            ReviewState::Resolved | ReviewState::Superseded => {
                (Vec::new(), vec![row, specific.clone()])
            }
        };
        check!(eq; actual, expected);
    }
    let mut nonempty = case("member-specific", "a", ReviewState::Pending);
    nonempty.member_ids.push(subject("a"));
    let mut other = case("other-family", "a", ReviewState::Pending);
    other.family = "school_identity".into();
    let outside = case("outside", "outside-member-union", ReviewState::Pending);
    let input = vec![nonempty, other, outside, specific];
    check!(eq; separate_generic_identity_cases(input.clone(), &[]), (Vec::new(), input));
    Ok(())
}

#[test]
fn only_absent_or_unaccepted_exact_insufficient_verdicts_allow_generic_partition() -> TestResult {
    let generic = case("generic", "a", ReviewState::Pending);
    let specific = resolved("specific", &["a"]);
    for (accepted, kind, expected_generic) in [
        (false, "insufficient_evidence", true),
        (true, "insufficient_evidence", false),
        (false, "value_proposed", false),
        (true, "value_proposed", false),
        (false, "Insufficient_Evidence", false),
        (false, "", false),
    ] {
        let result = separate_generic_identity_cases(
            vec![generic.clone(), specific.clone()],
            &[verdict("generic", accepted, kind)],
        );
        let expected = if expected_generic {
            (vec![generic.clone()], vec![specific.clone()])
        } else {
            (Vec::new(), vec![generic.clone(), specific.clone()])
        };
        check!(eq; result, expected);
    }
    check!(eq; separate_generic_identity_cases(vec![generic.clone(), specific.clone()], &[]), (vec![generic], vec![specific]));
    Ok(())
}

#[test]
fn verdict_lookup_is_by_case_id_and_preserves_last_duplicate_wins_semantics() -> TestResult {
    let generic = case("generic", "a", ReviewState::Retained);
    let specific = resolved("specific", &["a"]);
    let retain = verdict("generic", true, "insufficient_evidence");
    let insufficient = verdict("generic", false, "insufficient_evidence");
    let input = vec![generic.clone(), specific.clone()];
    check!(eq; separate_generic_identity_cases(input.clone(), &[retain.clone(), insufficient.clone()]), (vec![generic], vec![specific]));
    check!(eq; separate_generic_identity_cases(input.clone(), &[insufficient, retain]), (Vec::new(), input));
    Ok(())
}

#[test]
fn verdicts_for_other_cases_neither_block_members_nor_create_membership() -> TestResult {
    let generic = case("generic", "a", ReviewState::Retained);
    let specific = resolved("specific", &["a"]);
    let unrelated = verdict("unrelated", true, "value_proposed");
    let result =
        separate_generic_identity_cases(vec![generic.clone(), specific.clone()], &[unrelated]);
    check!(eq; result, (vec![generic], vec![specific]));
    Ok(())
}

#[test]
fn duplicate_cases_are_preserved_in_each_partition_without_silent_deduplication() -> TestResult {
    let generic = case("generic", "a", ReviewState::Retained);
    let specific = resolved("specific", &["a"]);
    let input = vec![
        generic.clone(),
        specific.clone(),
        generic.clone(),
        specific.clone(),
    ];
    check!(eq; separate_generic_identity_cases(input, &[]), (vec![generic.clone(), generic], vec![specific.clone(), specific]));
    Ok(())
}

#[test]
fn reversing_input_reverses_only_each_partitions_order_without_mutating_records() -> TestResult {
    let a = case("a", "a", ReviewState::Pending);
    let outside = case("outside", "outside", ReviewState::Retained);
    let b = case("b", "b", ReviewState::Retained);
    let specific = resolved("specific", &["a", "b"]);
    let input = vec![a.clone(), outside.clone(), b.clone(), specific.clone()];
    let (generic, bound) = separate_generic_identity_cases(input.clone(), &[]);
    check!(eq; generic, vec![a, b]);
    check!(eq; bound, vec![outside, specific]);
    let (mut reverse_generic, mut reverse_bound) =
        separate_generic_identity_cases(input.into_iter().rev().collect(), &[]);
    reverse_generic.reverse();
    reverse_bound.reverse();
    check!(eq; reverse_generic, generic);
    check!(eq; reverse_bound, bound);
    Ok(())
}
