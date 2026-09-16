use super::*;
use crate::domain::{
    evidence::{EvidenceRef, GradeAtSeason, Observed, ProfileEvidence, Sport, TeamEvidence},
    facts::{AthleteName, CityName, GraduationYear, Location, RegionName, SchoolName},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use std::collections::BTreeMap;

fn digest() -> EvidenceDigest {
    EvidenceDigest::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("synthetic digest")
}

fn evidence() -> EvidenceRef {
    EvidenceRef {
        document: digest(),
        locator: "fixture".to_owned(),
    }
}

fn observed<T>(value: T) -> Observed<T> {
    Observed {
        value,
        evidence: evidence(),
    }
}

fn source(first: &str, last: &str, school: Option<&str>, city: &str) -> SourceRecord {
    let fields = [
        ("Person First", first),
        ("Person Last", last),
        ("Address Mailing / Permanent City", city),
        ("Address Mailing / Permanent Region", "TX"),
    ]
    .into_iter()
    .chain(school.map(|value| ("Schools Name", value)))
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    SourceRecord {
        source_key: "Sheet:2".to_owned(),
        sheet: "Sheet".to_owned(),
        excel_row: 2,
        fields,
    }
}

fn profile(id: u64, name: &str, school: &str, city: &str, year: Option<u16>) -> ProfileEvidence {
    let athlete_id = AthleteId::new(id).expect("id");
    let profile_url = ProfileUrl::parse(&format!(
        "https://athletic.net/athlete/{id}/track-and-field"
    ))
    .expect("url");
    let team = TeamEvidence {
        team_id: id,
        name: observed(SchoolName::parse(school).expect("school")),
        location: Some(observed(Location::CityRegion {
            city: CityName::parse(city).expect("city"),
            region: RegionName::parse("TX").expect("region"),
        })),
        seasons: vec![2025],
        level: Some(1),
    };
    let graduation_years = year
        .into_iter()
        .map(|value| observed(GraduationYear::new(value).expect("year")))
        .collect();
    ProfileEvidence {
        athlete_id,
        profile_url,
        name: observed(AthleteName::parse(name).expect("name")),
        teams: vec![team],
        graduation_years,
        grades: Vec::new(),
        sports: vec![
            crate::domain::evidence::SportAvailability::ResultsObserved {
                sport: Sport::TrackField,
                count: 1,
            },
        ],
        results: vec![participation(id)],
        issues: Vec::new(),
        documents: vec![digest()],
    }
}

fn participation(id: u64) -> crate::domain::evidence::ResultEvidence {
    use crate::domain::evidence::{BestClaim, ResultAttribution, ResultEvidence};
    ResultEvidence {
        result_id: id,
        sport: Sport::TrackField,
        event_id: Some(1),
        event_name: "100 Meters".to_owned(),
        event_description: None,
        event_type: Some("T".to_owned()),
        mark: "12.30a".to_owned(),
        units: Some("s".to_owned()),
        season: 2026,
        team_id: id,
        meet_id: 1,
        meet_name: Some("Synthetic Meet".to_owned()),
        date: None,
        wind: None,
        timing: Some("FAT".to_owned()),
        personal_best: BestClaim::Unavailable,
        season_best: BestClaim::Unavailable,
        attribution: ResultAttribution::Individual,
        short_code: None,
        result_url: None,
        evidence: evidence(),
    }
}

fn complete() -> SearchCompleteness {
    SearchCompleteness::Complete { evidence: digest() }
}

#[test]
fn confident_bypass_requires_all_independent_corrobation() {
    let assessment = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[profile(
            7,
            "Ada Runner",
            "Central High",
            "Austin",
            Some(2027),
        )],
        complete(),
    )
    .expect("assessment");
    assert_eq!(assessment.decision(), Decision::DeterministicAccepted);
    assert_eq!(
        assessment.accepted_athlete_id().map(AthleteId::get),
        Some(7)
    );
    assert_eq!(assessment.candidates()[0].evidence_strength(), 100);
}

#[test]
fn duplicate_sports_are_one_identity_and_two_candidates_are_unresolved() {
    let tf = profile(7, "Ada Runner", "Central High", "Austin", Some(2027));
    let xc = profile(7, "Ada Runner", "Central High", "Austin", Some(2027));
    let other = profile(8, "Ada Runner", "Central High", "Austin", Some(2027));
    let assessment = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[tf, xc, other],
        complete(),
    )
    .expect("assessment");
    assert_eq!(assessment.candidates().len(), 2);
    assert_eq!(assessment.decision(), Decision::IdentityReview);
    assert!(apply_review(
        &assessment,
        ReviewChoice::Select(AthleteId::new(7).expect("id"))
    )
    .is_err());
    assert_eq!(
        apply_review(&assessment, ReviewChoice::Unresolved)
            .expect("review")
            .accepted_athlete_id(),
        None
    );
}

#[test]
fn workbook_membership_accepts_corroborated_identity_without_a_cohort_requirement() {
    let mut contradictory = profile(7, "Ada Runner", "Central High", "Austin", Some(2027));
    contradictory
        .graduation_years
        .push(observed(GraduationYear::new(2026).expect("year")));
    contradictory
        .issues
        .push(crate::domain::evidence::EvidenceIssue {
            code: "cohort_conflict".to_owned(),
            message: "descriptive years disagree".to_owned(),
            evidence: Some(evidence()),
        });
    let cases = [
        profile(7, "Ada Runner", "Central High", "Austin", None),
        profile(7, "Ada Runner", "Central High", "Austin", Some(2026)),
        contradictory,
    ];
    for candidate in cases {
        let assessment = assess(
            &source("Ada", "Runner", Some("Central High"), "Austin"),
            &[candidate],
            complete(),
        )
        .expect("assessment");
        assert_eq!(assessment.decision(), Decision::DeterministicAccepted);
        assert_eq!(
            assessment.accepted_athlete_id().map(AthleteId::get),
            Some(7)
        );
    }
}

#[test]
fn incomplete_search_never_claims_match_or_no_match() {
    let search = SearchCompleteness::Incomplete {
        reasons: vec!["page cap".to_owned()],
    };
    let assessment = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[profile(
            7,
            "Ada Runner",
            "Central High",
            "Austin",
            Some(2027),
        )],
        search,
    )
    .expect("assessment");
    assert_eq!(assessment.decision(), Decision::EvidenceReview);
    assert!(apply_review(
        &assessment,
        ReviewChoice::Select(AthleteId::new(7).expect("id"))
    )
    .is_err());
    let none = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[],
        SearchCompleteness::Incomplete {
            reasons: Vec::new(),
        },
    )
    .expect("assessment");
    assert_eq!(none.decision(), Decision::EvidenceReview);
}

#[test]
fn source_labels_do_not_contaminate_name_and_central_is_not_west_central() {
    let mut fields = BTreeMap::new();
    fields.insert(
        "Origin Source".to_owned(),
        "Ada Runner Central High".to_owned(),
    );
    let contaminated = SourceRecord {
        fields,
        ..SourceRecord::default()
    };
    let assessment = assess(&contaminated, &[], complete()).expect("assessment");
    assert_eq!(assessment.decision(), Decision::EvidenceReview);
    let assessment = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[profile(
            7,
            "Ada Runner",
            "West Central High",
            "Austin",
            Some(2027),
        )],
        complete(),
    )
    .expect("assessment");
    assert_eq!(assessment.decision(), Decision::CompleteSearchNoMatch);
}

#[test]
fn missing_school_and_distant_mailing_location_have_distinct_handling() {
    let missing = assess(
        &source("Ada", "Runner", None, "Austin"),
        &[profile(
            7,
            "Ada Runner",
            "Central High",
            "Austin",
            Some(2027),
        )],
        complete(),
    )
    .expect("assessment");
    assert_eq!(missing.decision(), Decision::EvidenceReview);
    let mut boarding = source("Ada", "Runner", Some("Central High"), "Boston");
    boarding.fields.insert(
        "Address Mailing / Permanent Region".to_owned(),
        "MA".to_owned(),
    );
    let distant = assess(
        &boarding,
        &[profile(
            7,
            "Ada Runner",
            "Central High",
            "Austin",
            Some(2027),
        )],
        complete(),
    )
    .expect("assessment");
    assert_eq!(distant.decision(), Decision::EvidenceReview);
    assert_eq!(distant.accepted_athlete_id(), None);
}

#[test]
fn grade_does_not_gate_a_valid_identity_and_forged_selection_is_rejected() {
    let mut candidate = profile(7, "Ada Runner", "Central High", "Austin", None);
    candidate.grades.push(GradeAtSeason {
        team_id: 7,
        season: 2025,
        grade: 11,
        evidence: evidence(),
    });
    let assessment = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[candidate],
        complete(),
    )
    .expect("assessment");
    assert_eq!(assessment.decision(), Decision::DeterministicAccepted);
    assert!(apply_review(
        &assessment,
        ReviewChoice::Select(AthleteId::new(999).expect("id"))
    )
    .is_err());
    assert_eq!(
        apply_review(
            &assessment,
            ReviewChoice::Select(AthleteId::new(7).expect("id"))
        )
        .expect("eligible identity")
        .accepted_athlete_id()
        .map(AthleteId::get),
        Some(7)
    );
}

#[test]
fn unrelated_multiline_source_fields_do_not_invalidate_identity() {
    let mut original = source("Ada", "Runner", Some("Central High"), "Austin");
    original.fields.insert(
        "Address Mailing / Permanent Street Combined".to_owned(),
        "12 Example Road\nUnit 2".to_owned(),
    );
    let assessment = assess(
        &original,
        &[profile(
            7,
            "Ada Runner",
            "Central High",
            "Austin",
            Some(2027),
        )],
        complete(),
    )
    .expect("assessment");
    assert_eq!(
        assessment.accepted_athlete_id().map(AthleteId::get),
        Some(7)
    );
}

#[test]
fn same_city_name_does_not_override_a_different_region() {
    let mut original = source("Ada", "Runner", Some("Central High"), "Austin");
    original.fields.insert(
        "Address Mailing / Permanent Region".to_owned(),
        "OR".to_owned(),
    );
    let assessment = assess(
        &original,
        &[profile(
            7,
            "Ada Runner",
            "Central High",
            "Austin",
            Some(2027),
        )],
        complete(),
    )
    .expect("assessment");
    assert_eq!(assessment.decision(), Decision::EvidenceReview);
}

#[test]
fn graduation_year_cannot_break_an_otherwise_indistinguishable_identity_tie() {
    let candidates = [
        profile(7, "Ada Runner", "Central High", "Austin", Some(2027)),
        profile(8, "Ada Runner", "Central High", "Austin", Some(2025)),
    ];
    let assessment = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &candidates,
        complete(),
    )
    .expect("assessment");
    assert_eq!(assessment.decision(), Decision::IdentityReview);
    assert_eq!(assessment.accepted_athlete_id(), None);
    assert!(apply_review(
        &assessment,
        ReviewChoice::Select(AthleteId::new(7).expect("id"))
    )
    .is_err());
}

#[test]
fn identity_fields_without_participation_cannot_be_accepted() {
    let mut candidate = profile(7, "Ada Runner", "Central High", "Austin", Some(2027));
    candidate.results.clear();
    let result = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[candidate],
        complete(),
    )
    .expect("assessment");
    assert_eq!(result.decision(), Decision::EvidenceReview);
    assert!(apply_review(
        &result,
        ReviewChoice::Select(AthleteId::new(7).expect("id"))
    )
    .is_err());
}

#[test]
fn merged_identity_conflict_cannot_disappear_behind_a_matching_name() {
    let mut candidate = profile(7, "Ada Runner", "Central High", "Austin", Some(2027));
    candidate
        .issues
        .push(crate::domain::evidence::EvidenceIssue {
            code: "identity_conflict".to_owned(),
            message: "conflicting observations".to_owned(),
            evidence: Some(evidence()),
        });
    let result = assess(
        &source("Ada", "Runner", Some("Central High"), "Austin"),
        &[candidate],
        complete(),
    )
    .expect("assessment");
    assert_eq!(result.decision(), Decision::EvidenceReview);
}
