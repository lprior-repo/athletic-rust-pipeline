use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, CoachTenure, CoachTenureEvidence,
    Gender, GradYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod postal;

#[test]
fn recruiting_csv_selects_current_contacts_for_each_gender() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Synthetic High", "synthetichigh");
    store.append(Table::Schools, &school)?;
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
                school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
            },
            source: SourceRef::new("fixture", Some("https://example.invalid/staff".into())),
            source_sha256: "a".repeat(64),
            retrieved_at: "2026-09-01T00:00:00Z".into(),
            statement: "Current 2026-27 head coach".into(),
        });
        store.append(Table::Coaches, &coach)?;
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
        athlete
            .published_graduations
            .push(census_domain::model::PublishedGraduation {
                grad_year: GradYear::CO2027,
                source: SourceRef::id("fixture"),
            });
        store.append(Table::Athletes, &athlete)?;
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
    store.append(Table::Coaches, &former)?;
    store.flush()?;
    let data = dir.path().join("csv");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )?;
    let mut reader = ::csv::Reader::from_path(data.join("recruiting-co2027.csv"))?;
    let headers = reader.headers()?.clone();
    let gender = headers
        .iter()
        .position(|value| value == "gender")
        .ok_or("missing gender column")?;
    let coach = headers
        .iter()
        .position(|value| value == "head_track_coach")
        .ok_or("missing coach column")?;
    let email = headers
        .iter()
        .position(|value| value == "head_track_coach_email")
        .ok_or("missing email column")?;
    let rows: Vec<_> = reader.records().collect::<Result<_, _>>()?;
    let boys = rows
        .iter()
        .find(|row| {
            row.get(gender)
                .is_some_and(|value| value.eq_ignore_ascii_case("boys"))
        })
        .ok_or("missing boys row")?;
    let girls = rows
        .iter()
        .find(|row| {
            row.get(gender)
                .is_some_and(|value| value.eq_ignore_ascii_case("girls"))
        })
        .ok_or("missing girls row")?;
    check!(eq; boys.get(coach), Some("Boys coach"));
    check!(eq; boys.get(email), Some("boys@example.invalid"));
    check!(eq; girls.get(coach), Some("Girls coach"));
    check!(eq; girls.get(email), Some("girls@example.invalid"));
    Ok(())
}

#[test]
fn unresolved_same_name_candidates_remain_individually_addressable() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Synthetic High", "synthetichigh");
    store.append(Table::Schools, &school)?;
    let candidate_ids = ["published-row-prelim", "published-row-final"]
        .into_iter()
        .map(|source_row| -> TestResult<String> {
            let mut athlete = CanonicalAthlete::new(
                &school_id,
                "Synthetic Runner",
                GradYear::CO2027,
                Gender::Boys,
                SourceIdentity::new(SourceNamespace::Other("fixture".into()), source_row),
            );
            athlete.sports = vec![Sport::OutdoorTrack];
            athlete
                .published_graduations
                .push(census_domain::model::PublishedGraduation {
                    grad_year: GradYear::CO2027,
                    source: SourceRef::id("fixture"),
                });
            store.append(Table::Athletes, &athlete)?;
            Ok(athlete.id.to_string())
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(ne; candidate_ids[0], candidate_ids[1]);
    let data = dir.path().join("csv");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )?;
    let mut reader = ::csv::Reader::from_path(data.join("recruiting-co2027.csv"))?;
    let headers = reader.headers()?.clone();
    let id_column = headers
        .iter()
        .position(|header| header == "athlete_id")
        .ok_or("missing athlete id column")?;
    let status_column = headers
        .iter()
        .position(|header| header == "identity_status")
        .ok_or("missing identity status column")?;
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    let actual: std::collections::BTreeMap<_, _> = rows
        .iter()
        .map(|row| -> TestResult<_> {
            Ok((
                row.get(id_column)
                    .ok_or("missing athlete id cell")?
                    .to_owned(),
                row.get(status_column)
                    .ok_or("missing status cell")?
                    .to_owned(),
            ))
        })
        .collect::<TestResult<_>>()?;
    let expected = candidate_ids
        .into_iter()
        .map(|id| (id, "unverified".to_owned()))
        .collect();
    check!(eq; actual, expected);
    Ok(())
}
