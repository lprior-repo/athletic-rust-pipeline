mod common;

use census_domain::model::{
    ReviewCase, ReviewCaseFact, ReviewVerdict, ReviewVerdictKind, VerdictBatch,
    ATHLETE_IDENTITY_FAMILY,
};
use census_review::{triage, Adjudication, Admitted, ReviewFamily};
use common::{identity_case, packet_for, proposal};

#[test]
fn triage_keeps_adjudication_and_counts_dropped() {
    let case = identity_case();
    let other = ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        "ath_fixture02",
        "Casey Jones (Madison East)",
        "one row names two provider objects",
    );
    let packet = packet_for(&case, true, &[]).with_case(ReviewCaseFact {
        case_id: other.id.clone(),
        family: other.family.clone(),
        detail: other.detail.clone(),
    });
    let mut blank = proposal(&other, "same_person");
    blank.value = Some("   ".to_string());
    let batch = VerdictBatch {
        subject_id: case.subject_id.clone(),
        verdicts: vec![
            proposal(&case, "same_person"),
            ReviewVerdict {
                case_id: "Athlete identity:ghost:p2:abc".to_string(),
                kind: ReviewVerdictKind::ValueProposed,
                field: Some("identity".to_string()),
                value: Some("same_person".to_string()),
                confidence: 90,
                rationale: "ghost".to_string(),
            },
            proposal(&case, "same_person"),
            blank,
        ],
    };
    let (triaged, dropped) = triage(&packet, ReviewFamily::AthleteIdentity, batch);
    assert_eq!(dropped, 2);
    assert_eq!(triaged.len(), 2);
    assert_eq!(
        triaged[0].1,
        Adjudication::Decided(Admitted {
            field: "identity".to_string(),
            value: "same_person".to_string(),
        })
    );
    assert_eq!(triaged[1].1, Adjudication::Undecided);
}

#[test]
fn families_route_to_rust_adjudication() {
    for family in ReviewFamily::askable() {
        assert_eq!(ReviewFamily::from_label(family.label()), Some(family));
    }
    assert_eq!(ReviewFamily::askable().len(), 3);
    assert_eq!(ReviewFamily::AthleteIdentity.field(), "identity");
    assert_eq!(ReviewFamily::SchoolJurisdiction.field(), "state");
    assert_eq!(ReviewFamily::MeetJurisdiction.field(), "state");
    assert_eq!(
        ReviewFamily::parse("athlete"),
        Some(ReviewFamily::AthleteIdentity)
    );
    assert_eq!(
        ReviewFamily::parse("school"),
        Some(ReviewFamily::SchoolJurisdiction)
    );
    assert_eq!(
        ReviewFamily::parse("meet venue"),
        Some(ReviewFamily::MeetJurisdiction)
    );
    assert_eq!(ReviewFamily::parse("same_person"), None);
    assert_eq!(ReviewFamily::from_label("Nope"), None);
}

#[test]
fn verdict_spellings_parse_and_slugs_roundtrip() {
    let matrix: [(&str, Option<census_review::AthleteVerdict>); 9] = [
        (
            "same person",
            Some(census_review::AthleteVerdict::SamePerson),
        ),
        (
            "same_person",
            Some(census_review::AthleteVerdict::SamePerson),
        ),
        (
            "SAME-PERSON",
            Some(census_review::AthleteVerdict::SamePerson),
        ),
        (
            "different person",
            Some(census_review::AthleteVerdict::DifferentPerson),
        ),
        (
            "differentperson",
            Some(census_review::AthleteVerdict::DifferentPerson),
        ),
        (
            "Different_Person",
            Some(census_review::AthleteVerdict::DifferentPerson),
        ),
        (
            "insufficient_evidence",
            Some(census_review::AthleteVerdict::InsufficientEvidence),
        ),
        ("twins", None),
        ("", None),
    ];
    for (input, expected) in matrix {
        assert_eq!(
            census_review::AthleteVerdict::parse(input),
            expected,
            "{input}"
        );
    }
    assert_eq!(
        census_review::AthleteVerdict::SamePerson.slug(),
        "same_person"
    );
    assert_eq!(
        census_review::AthleteVerdict::DifferentPerson.slug(),
        "different_person"
    );
    assert_eq!(
        census_review::AthleteVerdict::InsufficientEvidence.slug(),
        "insufficient_evidence"
    );
}
