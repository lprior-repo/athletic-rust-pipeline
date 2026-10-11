#![allow(dead_code)]

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, Grade, ObservedGrade,
    ReviewCase, ReviewCaseFact, ReviewEvidenceFact, ReviewPacket, ReviewVerdict, ReviewVerdictKind,
    SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;
use census_review::Adjudication;
use census_store::{Store, Table};

pub fn identity_case() -> ReviewCase {
    ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        "ath_fixture01",
        "Jordan Smith (Madison West)",
        "two rows share one provider object",
    )
}

pub fn packet_for(case: &ReviewCase, positive: bool, flags: &[&str]) -> ReviewPacket {
    let mut packet = ReviewPacket::new(case.subject_id.clone(), case.subject.clone())
        .with_case(ReviewCaseFact {
            case_id: case.id.clone(),
            family: case.family.clone(),
            detail: case.detail.clone(),
        })
        .with_evidence(ReviewEvidenceFact::new(
            "census",
            "answer_field",
            "identity",
        ))
        .with_evidence(ReviewEvidenceFact::new(
            "census",
            "answer_values",
            "same_person | different_person",
        ));
    if positive {
        packet = packet.with_evidence(ReviewEvidenceFact::new(
            "census",
            "flag",
            "shared_source_identity: one provider id is on both rows",
        ));
    }
    for flag in flags {
        packet = packet.with_evidence(ReviewEvidenceFact::new(
            "census",
            "flag",
            format!("{flag}: deterministic contradiction"),
        ));
    }
    packet
}

pub fn proposal(case: &ReviewCase, answer: &str) -> ReviewVerdict {
    ReviewVerdict {
        case_id: case.id.clone(),
        kind: ReviewVerdictKind::ValueProposed,
        field: Some("identity".to_string()),
        value: Some(answer.to_string()),
        confidence: 70,
        rationale: "fixture rationale".to_string(),
    }
}

pub fn school_id(store: &Store, name: &str) -> SchoolId {
    let (school, id) = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name));
    store
        .append(Table::Schools, &school)
        .expect("school appends");
    id
}

pub fn athlete_row(
    store: &Store,
    school: &SchoolId,
    name: &str,
    gender: Gender,
    object: &str,
    grade: u8,
) -> CanonicalAthlete {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, object);
    let mut athlete = CanonicalAthlete::new(school, name, GradYear::CO2027, gender, source);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(grade).expect("fixture grade"),
        school_year: SchoolYear::new(2025).expect("2025 is a season"),
        source: SourceRef::new("wiaa_results", None),
    });
    store
        .append(Table::Athletes, &athlete)
        .expect("athlete appends");
    athlete
}

pub fn rationale_of(adjudication: Adjudication) -> Option<&'static str> {
    match adjudication {
        Adjudication::Refused(refusal) => refusal.rationale(),
        Adjudication::Decided(_) => Some("decided"),
        Adjudication::Undecided => Some("undecided"),
    }
}
