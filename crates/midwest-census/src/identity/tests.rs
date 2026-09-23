//! What the lane must guarantee before a model's answer can be recorded: the family decides the
//! field, validation decides the value, and a refusal is a visible outcome rather than a silent one.

use super::packets::{case_fact, fact};
use super::*;
use census_domain::model::{
    ReviewCase, ReviewPacket, ReviewVerdict, ReviewVerdictKind, VerdictBatch,
};

fn school_packet(state: Option<&str>) -> ReviewPacket {
    let mut packet = ReviewPacket::new("school:madison-west", "Madison West High School")
        .with_case(case_fact(&ReviewCase::pending(
            "School jurisdiction unresolved",
            "school:madison-west",
            "Madison West High School",
            "no jurisdiction from any source",
        )))
        .with_evidence(fact("name", "Madison West High School"))
        .with_evidence(fact("city", "Madison"))
        .with_evidence(fact("association", "WIAA"));
    if let Some(state) = state {
        packet = packet.with_evidence(fact("state", state));
    }
    packet
}

fn meet_packet(state: Option<&str>) -> ReviewPacket {
    let mut packet = ReviewPacket::new("meet:coon-rapids-2026", "Coon Rapids Invitational")
        .with_case(case_fact(&ReviewCase::pending(
            "Meet venue unresolved",
            "meet:coon-rapids-2026",
            "Coon Rapids Invitational (2026-05-01)",
            "no evidence placed the venue in a jurisdiction; filed under ??",
        )))
        .with_evidence(fact("name", "Coon Rapids Invitational"))
        .with_evidence(fact("date", "2026-05-01"));
    if let Some(state) = state {
        packet = packet.with_evidence(fact("state", state));
    }
    packet
}

fn school_proposal(field: &str, value: &str) -> ReviewVerdict {
    ReviewVerdict {
        case_id: "School jurisdiction unresolved:school:madison-west".to_string(),
        kind: ReviewVerdictKind::ValueProposed,
        field: Some(field.to_string()),
        value: Some(value.to_string()),
        confidence: 70,
        rationale: "the association is the WIAA".to_string(),
    }
}

fn meet_proposal(field: &str, value: &str) -> ReviewVerdict {
    ReviewVerdict {
        case_id: "Meet venue unresolved:meet:coon-rapids-2026".to_string(),
        kind: ReviewVerdictKind::ValueProposed,
        field: Some(field.to_string()),
        value: Some(value.to_string()),
        confidence: 80,
        rationale: "Coon Rapids is a Minnesota school".to_string(),
    }
}

#[test]
fn a_family_is_looked_up_by_the_label_the_store_retained() {
    assert_eq!(
        ReviewFamily::from_label("School jurisdiction unresolved"),
        Some(ReviewFamily::SchoolJurisdiction)
    );
    assert_eq!(
        ReviewFamily::from_label("Meet venue unresolved"),
        Some(ReviewFamily::MeetJurisdiction),
        "the workbook's label for the meet family is the venue wording"
    );
    assert_eq!(
        ReviewFamily::from_label("Athlete identity"),
        Some(ReviewFamily::AthleteIdentity),
        "the merge's conflict label is the label this family is asked under"
    );
    assert_eq!(
        ReviewFamily::from_label("Class-of-2027 cohort unverified"),
        None,
        "a family this lane does not ask about stays with the operator"
    );
    assert_eq!(ReviewFamily::SchoolJurisdiction.field(), "state");
    assert_eq!(ReviewFamily::MeetJurisdiction.field(), "state");
    assert_eq!(
        ReviewFamily::AthleteIdentity.field(),
        "identity",
        "the athlete family answers a decision, not a jurisdiction"
    );
}

#[test]
fn a_family_parses_from_the_cli_spelling() {
    assert_eq!(
        ReviewFamily::parse("school-jurisdiction"),
        Some(ReviewFamily::SchoolJurisdiction)
    );
    assert_eq!(
        ReviewFamily::parse("meet_venue"),
        Some(ReviewFamily::MeetJurisdiction)
    );
    assert_eq!(
        ReviewFamily::parse("Meet Venue Unresolved"),
        Some(ReviewFamily::MeetJurisdiction)
    );
    assert_eq!(
        ReviewFamily::parse("athlete-identity"),
        Some(ReviewFamily::AthleteIdentity)
    );
    assert_eq!(
        ReviewFamily::parse("athlete_identity"),
        Some(ReviewFamily::AthleteIdentity)
    );
    assert_eq!(ReviewFamily::parse("cohort"), None);
    assert_eq!(ReviewFamily::askable().len(), 3);
    assert!(
        ReviewFamily::askable().contains(&ReviewFamily::AthleteIdentity),
        "the lane must offer the athlete family, or it is asked about by nobody"
    );
}

/// The decision a value admits.
fn decided(field: &str, value: &str) -> Adjudication {
    Adjudication::Decided(Admitted {
        field: field.to_string(),
        value: value.to_string(),
    })
}

#[test]
fn a_covered_state_is_admitted_as_a_code_and_normalized_from_a_name() {
    let packet = school_packet(None);
    assert_eq!(
        validate(
            ReviewFamily::SchoolJurisdiction,
            &school_proposal("state", "wi"),
            &packet
        ),
        decided("state", "WI"),
        "WI is a jurisdiction this census covers"
    );
    assert_eq!(
        validate(
            ReviewFamily::SchoolJurisdiction,
            &school_proposal("state", "Wisconsin"),
            &packet
        ),
        decided("state", "WI"),
        "a spelled-out state is the same answer"
    );
}

#[test]
fn a_meets_jurisdiction_is_admitted_the_same_way() {
    assert_eq!(
        validate(
            ReviewFamily::MeetJurisdiction,
            &meet_proposal("state", "MN"),
            &meet_packet(None)
        ),
        decided("state", "MN"),
        "the case asks for the state the meet was never filed under"
    );
}

#[test]
fn a_state_that_is_not_a_jurisdiction_is_refused() {
    let packet = school_packet(None);
    assert_eq!(
        validate(
            ReviewFamily::SchoolJurisdiction,
            &school_proposal("state", "Westconsin"),
            &packet
        ),
        Adjudication::Refused(Refusal::InvalidValue)
    );
    assert_eq!(
        validate(
            ReviewFamily::MeetJurisdiction,
            &meet_proposal("state", "North Minnesota"),
            &meet_packet(None)
        ),
        Adjudication::Refused(Refusal::InvalidValue)
    );
}

#[test]
fn a_proposal_for_a_field_no_family_asks_about_is_refused() {
    let packet = school_packet(None);
    assert_eq!(
        validate(
            ReviewFamily::SchoolJurisdiction,
            &school_proposal("location", "Coon Rapids HS"),
            &packet
        ),
        Adjudication::Refused(Refusal::WrongField)
    );
    assert_eq!(
        validate(
            ReviewFamily::MeetJurisdiction,
            &meet_proposal("grad_year", "2027"),
            &meet_packet(None)
        ),
        Adjudication::Refused(Refusal::WrongField)
    );
    let mut unnamed = school_proposal("state", "WI");
    unnamed.field = Some("  ".to_string());
    assert_eq!(
        validate(ReviewFamily::SchoolJurisdiction, &unnamed, &packet),
        Adjudication::Refused(Refusal::WrongField)
    );
}

#[test]
fn a_subject_that_already_carries_a_jurisdiction_is_not_re_proposed() {
    let packet = school_packet(Some("MN"));
    assert_eq!(
        validate(
            ReviewFamily::SchoolJurisdiction,
            &school_proposal("state", "WI"),
            &packet
        ),
        Adjudication::Refused(Refusal::AlreadyResolved)
    );
    let meet = meet_packet(Some("MN"));
    assert_eq!(
        validate(
            ReviewFamily::MeetJurisdiction,
            &meet_proposal("state", "MN"),
            &meet
        ),
        Adjudication::Refused(Refusal::AlreadyResolved)
    );
}

#[test]
fn triage_keeps_a_refused_proposal_as_a_verdict_without_a_value() {
    let packet = school_packet(None);
    let batch = VerdictBatch {
        subject_id: "school:madison-west".to_string(),
        verdicts: vec![school_proposal("state", "Westconsin")],
    };
    let (triaged, dropped) = triage(&packet, ReviewFamily::SchoolJurisdiction, batch);
    assert_eq!(dropped, 0);
    assert_eq!(triaged.len(), 1);
    let (verdict, adjudication) = &triaged[0];
    assert_eq!(verdict.kind, ReviewVerdictKind::ValueProposed);
    assert_eq!(
        adjudication,
        &Adjudication::Refused(Refusal::InvalidValue),
        "the refused proposal is recorded without a value"
    );
}

#[test]
fn the_report_line_names_every_outcome() {
    let report = ReviewReport {
        asked: 10,
        accepted: 4,
        rejected: 1,
        insufficient: 3,
        dropped: 1,
        unanswered: 1,
        failed: 1,
    };
    assert_eq!(report.resolved(), 7);
    assert_eq!(
        report.summary(),
        "asked=10 accepted=4 rejected=1 insufficient=3 unanswered=1 dropped=1 failed=1"
    );
}

#[test]
fn a_pass_writes_verdicts_only_when_it_is_not_a_dry_run() {
    assert!(should_write(false, 1));
    assert!(!should_write(true, 1));
    assert!(!should_write(false, 0));
}
