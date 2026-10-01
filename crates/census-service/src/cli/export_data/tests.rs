use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, CoachTenure, CoachTenureEvidence,
    Gender, GradYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::Table;

#[test]
fn recruiting_csv_selects_current_contacts_for_each_gender() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("store")).unwrap();
    let (school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Synthetic High", "synthetichigh");
    store.append(Table::Schools, &school).unwrap();
    for (side, name, email) in [
        (Gender::Boys, "Boys coach", "boys@example.invalid"),
        (Gender::Girls, "Girls coach", "girls@example.invalid"),
    ] {
        let mut coach = CanonicalCoach::new(
            &id,
            name,
            Some(Sport::OutdoorTrack),
            side,
            CoachRole::HeadCoach,
        );
        coach.professional_email = Some(email.into());
        coach.tenure_evidence.push(CoachTenureEvidence {
            tenure: CoachTenure::Current {
                school_year: SchoolYear::new(2026).unwrap(),
            },
            source: SourceRef::new("fixture", Some("https://example.invalid/staff".into())),
            source_sha256: "a".repeat(64),
            retrieved_at: "2026-09-01T00:00:00Z".into(),
            statement: "Current 2026-27 head coach".into(),
        });
        store.append(Table::Coaches, &coach).unwrap();
        let mut athlete = CanonicalAthlete::new(
            &id,
            format!("{name} runner"),
            GradYear::CO2027,
            side,
            SourceIdentity::new(
                SourceNamespace::Other("fixture".into()),
                format!("{name}-runner"),
            ),
        );
        athlete.sports = vec![Sport::OutdoorTrack];
        store.append(Table::Athletes, &athlete).unwrap();
    }
    let mut former = CanonicalCoach::new(
        &id,
        "Former boys coach",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    former.professional_email = Some("former@example.invalid".into());
    former.tenure_evidence.push(CoachTenureEvidence {
        tenure: CoachTenure::Former {
            last_school_year: SchoolYear::new(2025),
        },
        source: SourceRef::new("fixture", Some("https://example.invalid/old-staff".into())),
        source_sha256: "b".repeat(64),
        retrieved_at: "2026-09-01T00:00:00Z".into(),
        statement: "Former head coach, last academic year 2025-26".into(),
    });
    store.append(Table::Coaches, &former).unwrap();
    store.flush().unwrap();
    let data = dir.path().join("csv");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )
    .unwrap();
    let mut reader = ::csv::Reader::from_path(data.join("recruiting-co2027.csv")).unwrap();
    let headers = reader.headers().unwrap().clone();
    let gender = headers.iter().position(|value| value == "gender").unwrap();
    let coach = headers
        .iter()
        .position(|value| value == "head_track_coach")
        .unwrap();
    let email = headers
        .iter()
        .position(|value| value == "head_track_coach_email")
        .unwrap();
    let rows: Vec<_> = reader.records().collect::<Result<_, _>>().unwrap();
    let boys = rows
        .iter()
        .find(|row| {
            row.get(gender)
                .is_some_and(|value| value.eq_ignore_ascii_case("boys"))
        })
        .unwrap();
    let girls = rows
        .iter()
        .find(|row| {
            row.get(gender)
                .is_some_and(|value| value.eq_ignore_ascii_case("girls"))
        })
        .unwrap();
    assert_eq!(boys.get(coach), Some("Boys coach"));
    assert_eq!(boys.get(email), Some("boys@example.invalid"));
    assert_eq!(girls.get(coach), Some("Girls coach"));
    assert_eq!(girls.get(email), Some("girls@example.invalid"));
}

#[test]
fn unresolved_same_name_candidates_remain_individually_addressable() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("store")).unwrap();
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Synthetic High", "synthetichigh");
    store.append(Table::Schools, &school).unwrap();
    let candidate_ids = ["published-row-prelim", "published-row-final"].map(|source_row| {
        let mut athlete = CanonicalAthlete::new(
            &school_id,
            "Synthetic Runner",
            GradYear::CO2027,
            Gender::Boys,
            SourceIdentity::new(SourceNamespace::Other("fixture".into()), source_row),
        );
        athlete.sports = vec![Sport::OutdoorTrack];
        store.append(Table::Athletes, &athlete).unwrap();
        athlete.id.to_string()
    });
    assert_ne!(candidate_ids[0], candidate_ids[1]);
    let data = dir.path().join("csv");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )
    .unwrap();
    let mut reader = ::csv::Reader::from_path(data.join("recruiting-co2027.csv")).unwrap();
    let headers = reader.headers().unwrap().clone();
    let id_column = headers
        .iter()
        .position(|header| header == "athlete_id")
        .unwrap();
    let status_column = headers
        .iter()
        .position(|header| header == "identity_status")
        .unwrap();
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    let actual: std::collections::BTreeMap<_, _> = rows
        .iter()
        .map(|row| {
            (
                row.get(id_column).unwrap().to_owned(),
                row.get(status_column).unwrap().to_owned(),
            )
        })
        .collect();
    let expected = candidate_ids
        .into_iter()
        .map(|id| (id, "unverified".to_owned()))
        .collect();
    assert_eq!(actual, expected);
}
