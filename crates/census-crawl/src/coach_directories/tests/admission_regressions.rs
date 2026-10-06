use super::*;
use census_domain::model::{
    CanonicalSchool, CoachRole, Evidence, Gender, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use std::collections::HashSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn emit(
    staff: serde_json::Value,
    teams: serde_json::Value,
) -> Result<CoachEmission, Box<dyn std::error::Error>> {
    let body = serde_json::to_vec(&serde_json::json!({
        "name": "Test High School", "shortCode": "TEST", "staff": staff, "teams": teams
    }))?;
    let summary = parse_summary(&body)?;
    let (_, school) = CanonicalSchool::new(
        UsJurisdiction::Wyoming,
        "Test High School",
        "test high school",
        None,
    );
    Ok(coach_entities(
        &summary,
        &school,
        "https://example.test/summary",
        "2026-09-30T00:00:00Z",
        census_domain::model::SchoolYear::new(2026).ok_or("valid test school year")?,
        &crate::net::cache::content_digest(&body),
    )?)
}

#[test]
fn rejected_team_placement_cannot_hide_an_eligible_staff_fallback() -> TestResult {
    let emission = emit(
        serde_json::json!([{
            "id": "a", "amrId": "ada-owner", "firstName": "Ada", "lastName": "Lovelace", "title": "Head Coach",
            "teamName": "Boys' Cross Country", "teamLevel": "Varsity", "emails": ["ada@school.edu"]
        }]),
        serde_json::json!([{"name": "Boys' Cross Country", "level": "JV", "coachProfileIds": ["a"]}]),
    )?;
    let rows: Vec<_> = emission
        .coaches
        .iter()
        .map(|coach| {
            (
                coach.name.as_str(),
                coach.sport,
                coach.gender,
                coach.role,
                coach.professional_email.as_deref(),
            )
        })
        .collect();
    check!(eq;
        rows,
        [(
            "Ada Lovelace",
            Some(Sport::CrossCountry),
            Gender::Boys,
            CoachRole::HeadCoach,
            Some("ada@school.edu")
        )]
    );
    check!(eq; emission.counters.dropped_levels.get("JV"), Some(&1));
    for coach in &emission.coaches {
        check!(eq;
            coach.source_identities,
            [
                SourceIdentity::new(SourceNamespace::association_school(SOURCE_ID), "ada-owner")
                    .with_url("https://example.test/summary")
            ]
        );
        check!(eq; coach.evidence, [summary_evidence()]);
    }
    Ok(())
}

#[test]
fn absent_provider_ids_do_not_collapse_unrelated_published_people() -> TestResult {
    let emission = emit(
        serde_json::json!([
            {"firstName": "Ada", "lastName": "Lovelace", "title": "Head Coach", "teamName": "Boys' Cross Country", "teamLevel": "Varsity", "emails": ["ada@school.edu"]},
            {"firstName": "Grace", "lastName": "Hopper", "title": "Head Coach", "teamName": "Girls' Cross Country", "teamLevel": "Varsity", "emails": ["grace@school.edu"]}
        ]),
        serde_json::json!([]),
    )?;
    let rows: HashSet<_> = emission
        .coaches
        .iter()
        .map(|coach| {
            (
                coach.name.as_str(),
                coach.gender,
                coach.professional_email.as_deref(),
            )
        })
        .collect();
    check!(eq;
        rows,
        HashSet::from([
            ("Ada Lovelace", Gender::Boys, Some("ada@school.edu")),
            ("Grace Hopper", Gender::Girls, Some("grace@school.edu")),
        ])
    );
    for coach in &emission.coaches {
        check!(eq; coach.source_identities, []);
        check!(eq; coach.evidence, [summary_evidence()]);
    }
    Ok(())
}

#[test]
fn indoor_and_outdoor_roles_survive_in_either_source_order() -> TestResult {
    for names in [
        ["Boys' Track, Indoor", "Boys' Track"],
        ["Boys' Track", "Boys' Track, Indoor"],
    ] {
        let teams: Vec<_> = names
            .iter()
            .map(|name| {
                serde_json::json!({
                    "name": name, "level": "Varsity", "coachProfileIds": ["a"]
                })
            })
            .collect();
        let emission = emit(
            serde_json::json!([{"id": "a", "firstName": "Ada", "lastName": "Lovelace", "title": "Head Coach"}]),
            serde_json::Value::Array(teams),
        )?;
        let sports: HashSet<_> = emission.coaches.iter().map(|coach| coach.sport).collect();
        check!(eq;
            sports,
            HashSet::from([Some(Sport::IndoorTrack), Some(Sport::OutdoorTrack)])
        );
        let rows: HashSet<_> = emission
            .coaches
            .iter()
            .map(|coach| (coach.name.as_str(), coach.sport, coach.gender, coach.role))
            .collect();
        check!(eq;
            rows,
            HashSet::from([
                (
                    "Ada Lovelace",
                    Some(Sport::IndoorTrack),
                    Gender::Boys,
                    CoachRole::HeadCoach
                ),
                (
                    "Ada Lovelace",
                    Some(Sport::OutdoorTrack),
                    Gender::Boys,
                    CoachRole::HeadCoach
                ),
            ])
        );
        for coach in &emission.coaches {
            check!(eq; coach.evidence, [summary_evidence()]);
        }
    }
    Ok(())
}

#[test]
fn admitted_team_placement_takes_priority_over_conflicting_staff_fallback() -> TestResult {
    for names in [
        ["Boys' Cross Country", "Girls' Track, Outdoor"],
        ["Girls' Track, Outdoor", "Boys' Cross Country"],
    ] {
        let teams: Vec<_> = names
            .iter()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "level": if *name == "Boys' Cross Country" { "JV" } else { "Varsity" },
                    "coachProfileIds": ["a"]
                })
            })
            .collect();
        let emission = emit(
            serde_json::json!([{
                "id": "a", "firstName": "Ada", "lastName": "Lovelace", "title": "Head Coach",
                "teamName": "Boys' Cross Country", "teamLevel": "Varsity", "emails": ["ada@school.edu"]
            }]),
            serde_json::Value::Array(teams),
        )?;
        let rows: Vec<_> = emission
            .coaches
            .iter()
            .map(|coach| {
                (
                    coach.name.as_str(),
                    coach.sport,
                    coach.gender,
                    coach.role,
                    coach.professional_email.as_deref(),
                )
            })
            .collect();
        check!(eq;
            rows,
            [(
                "Ada Lovelace",
                Some(Sport::OutdoorTrack),
                Gender::Girls,
                CoachRole::HeadCoach,
                Some("ada@school.edu")
            )]
        );
        check!(eq; emission.counters.dropped_levels.get("JV"), Some(&1));
    }
    Ok(())
}

#[test]
fn unidentified_same_name_staff_retain_their_separate_contacts_in_either_order() -> TestResult {
    for addresses in [
        ["ada@first.edu", "ada@second.edu"],
        ["ada@second.edu", "ada@first.edu"],
    ] {
        let staff: Vec<_> = addresses
            .iter()
            .map(|address| {
                serde_json::json!({
                    "id": "", "firstName": "Ada", "lastName": "Lovelace",
                    "title": "Head Coach", "teamName": "Boys' Cross Country",
                    "teamLevel": "Varsity", "emails": [address]
                })
            })
            .collect();
        let emission = emit(serde_json::Value::Array(staff), serde_json::json!([]))?;
        let contacts: HashSet<_> = emission
            .coaches
            .iter()
            .map(|coach| (coach.name.as_str(), coach.professional_email.as_deref()))
            .collect();
        check!(eq;
            contacts,
            HashSet::from([
                ("Ada Lovelace", Some("ada@first.edu")),
                ("Ada Lovelace", Some("ada@second.edu")),
            ])
        );
        for coach in &emission.coaches {
            check!(eq; coach.source_identities, []);
            check!(eq; coach.evidence, [summary_evidence()]);
        }
    }
    Ok(())
}

#[test]
fn duplicate_provider_records_apply_admission_before_contact_precedence() -> TestResult {
    for rejected in [
        serde_json::json!({"id":"a", "firstName":"Ada", "lastName":"Lovelace", "emails":["support@dragonflyathletics.com"]}),
        serde_json::json!({"id":"a", "firstName":"", "lastName":"", "emails":["other@school.edu"]}),
    ] {
        let emission = emit(
            serde_json::json!([
                {"id":"a", "firstName":"Ada", "lastName":"Lovelace", "emails":["ada@school.edu"]},
                rejected
            ]),
            serde_json::json!([{"name":"Boys' Cross Country", "level":"Varsity", "coachProfileIds":["a"]}]),
        )?;
        let rows: Vec<_> = emission
            .coaches
            .iter()
            .map(|coach| (coach.name.as_str(), coach.professional_email.as_deref()))
            .collect();
        check!(eq; rows, [("Ada Lovelace", Some("ada@school.edu"))]);
    }
    let emission = emit(
        serde_json::json!([
            {"id":"a", "firstName":"Ada", "lastName":"Lovelace", "emails":["old@school.edu"]},
            {"id":"a", "firstName":"Ada", "lastName":"Lovelace", "emails":["new@school.edu"]}
        ]),
        serde_json::json!([{"name":"Boys' Cross Country", "level":"Varsity", "coachProfileIds":["a"]}]),
    )?;
    let contacts: Vec<_> = emission
        .coaches
        .iter()
        .map(|coach| coach.professional_email.as_deref())
        .collect();
    check!(eq; contacts, [Some("new@school.edu")]);
    Ok(())
}

fn summary_evidence() -> Evidence {
    Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some("https://example.test/summary".to_string())),
        "2026-09-30T00:00:00Z",
    )
}
