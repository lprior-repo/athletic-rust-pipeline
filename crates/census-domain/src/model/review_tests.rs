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
fn a_packet_asks_about_exactly_the_cases_it_carries() -> Result<(), Box<dyn std::error::Error>> {
    let packet = packet();
    check!(eq; packet.case_ids(),
    vec!["School jurisdiction unresolved:school:madison-west"]);
    check!(eq; packet.evidence.len(), 1);
    check!(packet.evidence[0].value.contains("Madison West"));
    Ok(())
}

#[test]
fn verdicts_for_cases_the_packet_never_asked_about_are_dropped(
) -> Result<(), Box<dyn std::error::Error>> {
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
    check!(eq; dropped, 1, "a model may not invent work");
    check!(eq; admitted.len(), 1);
    check!(eq; admitted[0].value.as_deref(), Some("WI"));
    Ok(())
}

#[test]
fn one_verdict_per_case_is_kept_and_the_rest_are_counted_as_dropped(
) -> Result<(), Box<dyn std::error::Error>> {
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
    check!(eq; admitted.len(), 1);
    check!(eq; admitted[0].kind, ReviewVerdictKind::ValueProposed);
    check!(eq; dropped, 1);
    Ok(())
}

#[test]
fn a_proposal_without_a_value_is_demoted_rather_than_applied(
) -> Result<(), Box<dyn std::error::Error>> {
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
    check!(eq; dropped, 0);
    check!(eq; admitted[0].kind, ReviewVerdictKind::InsufficientEvidence);
    check!(eq; admitted[0].value, None);
    check!(eq; admitted[0].field, None);
    check!(!admitted[0].proposes_a_value());
    Ok(())
}

#[test]
fn a_confidence_outside_the_field_range_is_clamped_not_trusted(
) -> Result<(), Box<dyn std::error::Error>> {
    let batch = VerdictBatch {
        subject_id: "school:madison-west".to_string(),
        verdicts: vec![verdict(
            "School jurisdiction unresolved:school:madison-west",
            ReviewVerdictKind::ValueProposed,
            250,
        )],
    };
    let (admitted, dropped) = batch.sanitize(&packet());
    check!(eq; dropped, 0);
    check!(eq; admitted[0].confidence, 100);
    check!(admitted[0].proposes_a_value());
    Ok(())
}

#[test]
fn another_subject_cannot_supply_a_verdict_for_a_requested_case(
) -> Result<(), Box<dyn std::error::Error>> {
    for subject_id in ["", "school:another-school"] {
        let batch = VerdictBatch {
            subject_id: subject_id.to_string(),
            verdicts: vec![verdict(
                "School jurisdiction unresolved:school:madison-west",
                ReviewVerdictKind::ValueProposed,
                95,
            )],
        };
        let (admitted, dropped) = batch.sanitize(&packet());
        check!(admitted.is_empty());
        check!(eq; dropped, 1);
    }
    Ok(())
}

#[test]
fn model_reply_rejects_unknown_batch_and_verdict_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let batch = VerdictBatch {
        subject_id: "school:madison-west".to_string(),
        verdicts: vec![verdict(
            "School jurisdiction unresolved:school:madison-west",
            ReviewVerdictKind::ValueProposed,
            80,
        )],
    };
    let encoded = serde_json::to_value(&batch)?;
    for nested in [false, true] {
        let mut reply = encoded.clone();
        let object = if nested {
            reply["verdicts"][0].as_object_mut()
        } else {
            reply.as_object_mut()
        };
        object
            .ok_or("missing model reply object fixture")?
            .insert("unexpected".into(), serde_json::json!("unsupported advice"));
        check!(
            serde_json::from_value::<VerdictBatch>(reply).is_err(),
            "unsupported fields must not become accepted advice"
        );
    }
    check!(eq; serde_json::from_value::<VerdictBatch>(encoded)?, batch);
    Ok(())
}
