use super::*;

#[test]
fn matches_evidence_accepts_matching_digest() {
    let evidence = CaseEvidence::of(["subject", "detail"]);
    let case = ReviewCase::pending("Athlete identity", "sub1", "Subject", "detail");
    assert!(case.matches_evidence(&evidence));
}

#[test]
fn matches_evidence_rejects_mismatched_digest() {
    let case = ReviewCase::pending("Athlete identity", "sub1", "Subject", "detail");
    let different = CaseEvidence::of(["other_subject", "detail"]);
    assert!(!case.matches_evidence(&different));
}

#[test]
fn matches_evidence_rejects_empty_id() {
    let evidence = CaseEvidence::of(["subject", "detail"]);
    let case = ReviewCase {
        id: String::new(),
        family: "Athlete identity".to_string(),
        subject_id: "sub1".to_string(),
        subject: "Subject".to_string(),
        detail: "detail".to_string(),
        state: ReviewState::Pending,
        member_ids: Vec::new(),
    };
    assert!(!case.matches_evidence(&evidence));
}
