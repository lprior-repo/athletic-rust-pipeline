use super::*;
use crate::domain::{
    evidence::{
        BestClaim, EvidenceRef, GradeAtSeason, Observed, ProfileEvidence, ResultAttribution,
        ResultEvidence, Sport, TeamEvidence,
    },
    facts::{AthleteName, SchoolName},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use serde_json::Value;

fn digest() -> EvidenceDigest {
    EvidenceDigest::parse(&"a".repeat(64)).expect("synthetic digest")
}

fn evidence(locator: String) -> EvidenceRef {
    EvidenceRef {
        document: digest(),
        locator,
    }
}

fn result(id: u64, event_type: String, locator: String) -> ResultEvidence {
    ResultEvidence {
        result_id: id,
        sport: Sport::TrackField,
        event_id: Some(1),
        event_name: "100m".to_owned(),
        event_description: None,
        event_type: Some(event_type),
        mark: "10.20".to_owned(),
        units: Some("Seconds".to_owned()),
        season: 2026,
        team_id: 7,
        meet_id: 8,
        meet_name: Some("Synthetic Meet".to_owned()),
        date: Some("2026-05-01".to_owned()),
        wind: Some("1.0".to_owned()),
        timing: Some("FAT".to_owned()),
        personal_best: BestClaim::OpaqueFlags(2),
        season_best: BestClaim::OpaqueFlags(1),
        attribution: ResultAttribution::Individual,
        short_code: Some(format!("r{id}")),
        result_url: None,
        evidence: evidence(locator),
    }
}

fn profile(id: u64, results: Vec<ResultEvidence>) -> ProfileEvidence {
    let athlete_id = AthleteId::new(id).expect("synthetic athlete id");
    ProfileEvidence {
        athlete_id,
        profile_url: ProfileUrl::parse(&format!(
            "https://athletic.net/athlete/{id}/track-and-field"
        ))
        .expect("synthetic profile URL"),
        name: Observed {
            value: AthleteName::parse("Synthetic Runner").expect("synthetic athlete name"),
            evidence: evidence("/athlete/name".to_owned()),
        },
        teams: Vec::new(),
        graduation_years: Vec::new(),
        grades: Vec::new(),
        sports: Vec::new(),
        results,
        issues: Vec::new(),
        documents: Vec::new(),
    }
}

fn parse_summary(encoded: &str) -> Value {
    serde_json::from_str(encoded).expect("summary JSON")
}

#[test]
fn compact_summary_keeps_selected_best_provenance_and_opaque_claim_unknown() {
    let selected = profile(7, vec![result(101, "T".to_owned(), "/selected".to_owned())]);
    let unrelated = profile(
        8,
        vec![result(202, "T".to_owned(), "/unrelated".to_owned())],
    );
    let report_digest = digest();
    let encoded = build_pr_summary(
        &[selected, unrelated],
        AthleteId::new(7).expect("synthetic athlete id"),
        "Synthetic:2",
        &report_digest,
    )
    .expect("compact summary");
    let summary = parse_summary(&encoded);
    let best = summary["observed_best"].as_array().expect("best array");
    assert_eq!(best.len(), 1);
    assert_eq!(best[0]["result_id"], 101);
    assert_eq!(best[0]["evidence"]["locator"], "/selected");
    assert!(best[0].get("source").is_none());
    assert!(encoded.contains("timing"));
    assert!(encoded.contains("wind"));
    let claims = summary["source_personal_best_claims"]
        .as_array()
        .expect("claim array");
    assert_eq!(claims.len(), 1);
    assert!(claims[0]["claimed"].is_null());
    assert_eq!(claims[0]["raw"]["state"], "opaque_flags");
    assert!(!encoded.contains("202"));
}

#[test]
fn oversized_selected_profile_moves_summary_to_detail_sidecar_without_truncation() {
    let results = (1_u64..=1_200)
        .map(|id| {
            result(
                id,
                format!("synthetic-context-{id}"),
                format!("/results/{id}"),
            )
        })
        .collect::<Vec<_>>();
    let selected = profile(7, results);
    let report_digest = digest();
    let encoded = build_pr_summary(
        &[selected],
        AthleteId::new(7).expect("synthetic athlete id"),
        "Synthetic:2",
        &report_digest,
    )
    .expect("sidecar summary");
    assert!(encoded.encode_utf16().count() <= MAX_EXCEL_CELL_UTF16_UNITS);
    let summary = parse_summary(&encoded);
    assert_eq!(summary["storage"], "detail_sidecar");
    assert_eq!(summary["source_key"], "Synthetic:2");
    assert_eq!(summary["report_digest"], digest().as_str());
    assert_eq!(summary["selected_athlete_id"], 7);
    assert_eq!(summary["observed_best_group_count"], 1_200);
    assert!(summary["explanation"]
        .as_str()
        .is_some_and(|text| text.contains("JSONL")));
    assert!(summary.get("observed_best").is_none());
}

#[test]
fn roster_fields_report_newest_season_school_and_season_bound_grades() {
    let mut selected = profile(7, Vec::new());
    selected.teams.push(TeamEvidence {
        team_id: 11,
        name: Observed {
            value: SchoolName::parse("Old High").expect("synthetic school"),
            evidence: evidence("/old-school".to_owned()),
        },
        location: None,
        seasons: vec![2024, 2025],
        level: Some(1),
    });
    selected.teams.push(TeamEvidence {
        team_id: 12,
        name: Observed {
            value: SchoolName::parse("New High").expect("synthetic school"),
            evidence: evidence("/new-school".to_owned()),
        },
        location: None,
        seasons: vec![2026],
        level: Some(1),
    });
    selected.grades.push(GradeAtSeason {
        team_id: 12,
        season: 2026,
        grade: 11,
        evidence: evidence("/grade-2026".to_owned()),
    });
    selected.grades.push(GradeAtSeason {
        team_id: 11,
        season: 2025,
        grade: 10,
        evidence: evidence("/grade-2025".to_owned()),
    });
    let unrelated = profile(8, Vec::new());

    let annotations = annotations_for(
        &[selected, unrelated],
        Some(AthleteId::new(7).expect("synthetic athlete id")),
    );
    assert_eq!(annotations.competing_school, "New High");
    assert_eq!(
        annotations.school_history,
        "2026 New High; 2025 Old High; 2024 Old High"
    );
    assert_eq!(
        annotations.junior_evidence,
        "grade 11 @ 2026; grade 10 @ 2025"
    );
    assert_eq!(annotations.junior_status, "junior @ 2026");
    assert_eq!(
        annotations.profile_url,
        "https://athletic.net/athlete/7/track-and-field"
    );
    assert_eq!(annotations_for(&[], None), AcceptedAnnotations::default());

    let mut unnamed_grade = profile(9, Vec::new());
    unnamed_grade.grades.push(GradeAtSeason {
        team_id: 1,
        season: 2027,
        grade: 13,
        evidence: evidence("/grade-2027".to_owned()),
    });
    let verdict = annotations_for(
        &[unnamed_grade],
        Some(AthleteId::new(9).expect("synthetic athlete id")),
    );
    assert_eq!(verdict.junior_status, "");
    assert_eq!(verdict.junior_evidence, "grade 13 @ 2027");
}
