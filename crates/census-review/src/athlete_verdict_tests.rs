//! What the athlete family's reader must accept and refuse: exactly three answers, a verdict that
//! names the case it decided, and a proposal that either parses or is refused with the value kept.
//!
//! The reader is fed wire verdicts, not typed ones: it is the boundary a model's answer crosses, so
//! these are the spellings and shapes a server can actually return.

use census_domain::model::{ReviewCase, ReviewPacket, ReviewVerdict, ReviewVerdictKind};

use crate::packets::{case_fact, fact};
use crate::verdicts::{validate, Adjudication, Admitted, Refusal};
use crate::ReviewFamily;

use super::{read, AthleteVerdict};

/// The retained case: one canonical athlete, and the ids the store kept it apart from.
fn case() -> ReviewCase {
    ReviewCase::pending(
        "Athlete identity",
        "ath_1a2b3c4d",
        "Jordan Smith (Madison West High School)",
        "same school, name and cohort as every id here: ath_1a2b3c4d, ath_5e6f7a8b",
    )
}

/// The packet the case is asked with.
fn packet() -> ReviewPacket {
    let case = case();
    ReviewPacket::new(case.subject_id.clone(), case.subject.clone())
        .with_case(case_fact(&case))
        .with_evidence(fact("answer_field", "identity"))
}

/// A proposal carrying one answer, as a model would return it.
fn proposal(answer: &str) -> ReviewVerdict {
    ReviewVerdict {
        case_id: case().id,
        kind: ReviewVerdictKind::ValueProposed,
        field: Some("identity".to_string()),
        value: Some(answer.to_string()),
        confidence: 70,
        rationale: "one provider id is on both rows".to_string(),
    }
}

#[test]
fn each_of_the_three_answers_reads_as_this_familys_verdict() {
    assert_eq!(
        read(&proposal("same_person"), &packet()),
        Ok(AthleteVerdict::SamePerson)
    );
    assert_eq!(
        read(&proposal("different_person"), &packet()),
        Ok(AthleteVerdict::DifferentPerson)
    );
    assert_eq!(
        read(&proposal("insufficient_evidence"), &packet()),
        Ok(AthleteVerdict::InsufficientEvidence),
        "a model that declines in the value slot has still declined"
    );
    // The protocol's own way of declining carries no field and no value at all.
    let declined = ReviewVerdict {
        case_id: case().id,
        kind: ReviewVerdictKind::InsufficientEvidence,
        field: None,
        value: None,
        confidence: 30,
        rationale: "the evidence does not decide".to_string(),
    };
    assert_eq!(
        read(&declined, &packet()),
        Ok(AthleteVerdict::InsufficientEvidence)
    );
}

#[test]
fn the_two_decisions_validate_and_the_decline_closes_nothing() {
    assert_eq!(
        validate(
            ReviewFamily::AthleteIdentity,
            &proposal("same_person"),
            &packet()
        ),
        Adjudication::Decided(Admitted {
            field: "identity".to_string(),
            value: "same_person".to_string(),
        })
    );
    assert_eq!(
        validate(
            ReviewFamily::AthleteIdentity,
            &proposal("different_person"),
            &packet()
        ),
        Adjudication::Decided(Admitted {
            field: "identity".to_string(),
            value: "different_person".to_string(),
        })
    );
    assert_eq!(
        validate(
            ReviewFamily::AthleteIdentity,
            &proposal("insufficient_evidence"),
            &packet()
        ),
        Adjudication::Undecided,
        "a decline is an answer, but it is not a decision"
    );
    assert!(AthleteVerdict::SamePerson.decides());
    assert!(AthleteVerdict::DifferentPerson.decides());
    assert!(!AthleteVerdict::InsufficientEvidence.decides());
}

#[test]
fn an_answer_that_is_not_one_of_the_three_is_refused_with_the_value_it_carried() {
    assert_eq!(
        read(&proposal("maybe_same"), &packet()),
        Err(Refusal::InvalidValue)
    );
    assert_eq!(
        validate(
            ReviewFamily::AthleteIdentity,
            &proposal("maybe_same"),
            &packet()
        ),
        Adjudication::Refused(Refusal::InvalidValue),
        "the refusal names the rule, and the verdict keeps the value the model gave"
    );
}

#[test]
fn a_proposal_that_answers_another_field_is_refused() {
    let mut wrong = proposal("same_person");
    wrong.field = Some("state".to_string());
    assert_eq!(read(&wrong, &packet()), Err(Refusal::WrongField));

    let mut unnamed = proposal("same_person");
    unnamed.field = Some("  ".to_string());
    assert_eq!(read(&unnamed, &packet()), Err(Refusal::WrongField));
}

#[test]
fn a_verdict_that_names_another_case_decides_nothing() {
    let mut elsewhere = proposal("same_person");
    elsewhere.case_id = "School jurisdiction unresolved:school:madison-west".to_string();
    assert_eq!(read(&elsewhere, &packet()), Err(Refusal::UnnamedCase));

    let mut nameless = proposal("same_person");
    nameless.case_id = String::new();
    assert_eq!(read(&nameless, &packet()), Err(Refusal::UnnamedCase));
}

#[test]
fn the_three_answers_parse_in_the_spellings_a_model_drifts_between() {
    assert_eq!(
        AthleteVerdict::parse("SAME PERSON"),
        Some(AthleteVerdict::SamePerson)
    );
    assert_eq!(
        AthleteVerdict::parse(" Different-Person "),
        Some(AthleteVerdict::DifferentPerson)
    );
    assert_eq!(
        AthleteVerdict::parse("insufficient evidence"),
        Some(AthleteVerdict::InsufficientEvidence)
    );
    assert_eq!(
        AthleteVerdict::parse("same_person").map(AthleteVerdict::slug),
        Some("same_person"),
        "the slug a decision records is the spelling the packet asked for"
    );
    assert_eq!(AthleteVerdict::parse("twins"), None);
}
