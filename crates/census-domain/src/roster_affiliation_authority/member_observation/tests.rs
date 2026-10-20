use super::*;
use crate::model::SourceRef;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn evidence(note: String) -> Evidence {
    let mut evidence = Evidence::parsed(SourceRef::id("roster-test"), "2026-10-20T00:00:00Z");
    evidence.note = Some(note);
    evidence
}

#[test]
fn bounded_unrelated_and_unstructured_evidence_remain_unclassified() -> TestResult {
    for note in [r#"{"role":"other_role"}"#, "not a structured roster note"] {
        check!(matches!(
            RosterMemberObservationV1::read(&evidence(note.to_string())),
            Ok(None)
        ));
    }
    Ok(())
}

#[test]
fn unrelated_note_classifier_admits_the_ceiling_and_refuses_one_extra_byte() -> TestResult {
    let mut note = r#"{"role":"other_role"}"#.to_string();
    note.push_str(&" ".repeat(16 * 1024 - note.len()));
    check!(matches!(
        RosterMemberObservationV1::read(&evidence(note.clone())),
        Ok(None)
    ));
    note.push(' ');
    check!(matches!(
        RosterMemberObservationV1::read(&evidence(note)),
        Err(RosterAuthorityError::Invalid(_))
    ));
    Ok(())
}

#[test]
fn oversized_recognized_notes_refuse_even_when_json_is_incomplete() -> TestResult {
    let padding = " ".repeat(17 * 1024);
    for note in [
        format!(r#"{{"role":"roster_positive_row_metadata_v1"}}{padding}"#),
        format!(r#"{{"role":"roster_positive_row_metadata_v1","incomplete":"{padding}"#),
    ] {
        check!(matches!(
            RosterMemberObservationV1::read(&evidence(note)),
            Err(RosterAuthorityError::Invalid(_))
        ));
    }
    Ok(())
}
