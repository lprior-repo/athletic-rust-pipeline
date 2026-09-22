//! What the review lane's vocabulary must guarantee: a model cannot widen the question, widen its
//! answer, or claim a proposal it did not make.

use super::*;

fn packet() -> ReviewPacket {
    ReviewPacket::new("school:madison-west", "Madison West High School")
        .with_case(ReviewCaseFact {
            case_id: "School jurisdiction unresolved:school:madison-west".to_string(),
            family: "School jurisdiction unresolved".to_string(),
            detail: "no state from any source".to_string(),
        })
        .with_evidence(ReviewEvidenceFact::new(
            "wiaa_school",
            "name",
            "Madison West High School",
        ))
}

fn verdict(case_id: &str, kind: ReviewVerdictKind, confidence: u8) -> ReviewVerdict {
    ReviewVerdict {
        case_id: case_id.to_string(),
        kind,
        field: Some("state".to_string()),
        value: Some("WI".to_string()),
        confidence,
        rationale: "the school plays in the WIAA membership list".to_string(),
    }
}

#[test]
fn a_packet_asks_about_exactly_the_cases_it_carries() {
    let packet = packet();
    assert_eq!(
        packet.case_ids(),
        vec!["School jurisdiction unresolved:school:madison-west"]
    );
    assert_eq!(packet.evidence.len(), 1);
    assert!(packet.evidence[0].value.contains("Madison West"));
}

#[test]
fn verdicts_for_cases_the_packet_never_asked_about_are_dropped() {
    let batch = VerdictBatch {
        subject_id: "school:madison-west".to_string(),
        verdicts: vec![
            verdict(
                "School jurisdiction unresolved:school:madison-west",
                ReviewVerdictKind::ValueProposed,
                90,
            ),
            verdict(
                "invented_case:school:madison-west",
                ReviewVerdictKind::ValueProposed,
                99,
            ),
        ],
    };
    let (admitted, dropped) = batch.sanitize(&packet());
    assert_eq!(dropped, 1, "a model may not invent work");
    assert_eq!(admitted.len(), 1);
    assert_eq!(admitted[0].value.as_deref(), Some("WI"));
}

#[test]
fn one_verdict_per_case_is_kept_and_the_rest_are_counted_as_dropped() {
    let batch = VerdictBatch {
        subject_id: "school:madison-west".to_string(),
        verdicts: vec![
            verdict(
                "School jurisdiction unresolved:school:madison-west",
                ReviewVerdictKind::ValueProposed,
                90,
            ),
            verdict(
                "School jurisdiction unresolved:school:madison-west",
                ReviewVerdictKind::InsufficientEvidence,
                10,
            ),
        ],
    };
    let (admitted, dropped) = batch.sanitize(&packet());
    assert_eq!(admitted.len(), 1);
    assert_eq!(admitted[0].kind, ReviewVerdictKind::ValueProposed);
    assert_eq!(dropped, 1);
}

#[test]
fn a_proposal_without_a_value_is_demoted_rather_than_applied() {
    let mut empty = verdict(
        "School jurisdiction unresolved:school:madison-west",
        ReviewVerdictKind::ValueProposed,
        95,
    );
    empty.value = Some("   ".to_string());
    let batch = VerdictBatch {
        subject_id: "school:madison-west".to_string(),
        verdicts: vec![empty],
    };
    let (admitted, dropped) = batch.sanitize(&packet());
    assert_eq!(dropped, 0);
    assert_eq!(admitted[0].kind, ReviewVerdictKind::InsufficientEvidence);
    assert_eq!(admitted[0].value, None);
    assert_eq!(admitted[0].field, None);
    assert!(!admitted[0].proposes_a_value());
}

#[test]
fn a_confidence_outside_the_field_range_is_clamped_not_trusted() {
    let batch = VerdictBatch {
        subject_id: "school:madison-west".to_string(),
        verdicts: vec![verdict(
            "School jurisdiction unresolved:school:madison-west",
            ReviewVerdictKind::ValueProposed,
            250,
        )],
    };
    let (admitted, dropped) = batch.sanitize(&packet());
    assert_eq!(dropped, 0);
    assert_eq!(admitted[0].confidence, 100);
    assert!(admitted[0].proposes_a_value());
}

#[test]
fn the_two_kinds_parse_from_their_slugs_and_their_spaced_spellings() {
    assert_eq!(
        ReviewVerdictKind::parse("value_proposed"),
        Some(ReviewVerdictKind::ValueProposed)
    );
    assert_eq!(
        ReviewVerdictKind::parse("Value Proposed"),
        Some(ReviewVerdictKind::ValueProposed)
    );
    assert_eq!(
        ReviewVerdictKind::parse("insufficient evidence"),
        Some(ReviewVerdictKind::InsufficientEvidence)
    );
    assert_eq!(
        ReviewVerdictKind::parse("insufficient_evidence"),
        Some(ReviewVerdictKind::InsufficientEvidence)
    );
    assert_eq!(ReviewVerdictKind::parse("probably"), None);
    assert_eq!(ReviewVerdictKind::ValueProposed.slug(), "value_proposed");
}
