use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachContactClaim, CoachContactProgram,
    CoachRole, CoachTenure, CoachTenureEvidence, Evidence, Gender, GradYear, PublishedGraduation,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use crate::export::ExportDataset;
use crate::workbook::Options;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod bests;
mod postal;

fn seed(store: &Store) -> TestResult {
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Ada High",
        "ada high",
        Some("Madison"),
    );
    school.athletics_website = Some("https://ada.test/athletics".to_string());
    school.evidence = vec![Evidence::parsed(
        SourceRef::new("wiaa_results", Some("https://wiaa.test/ada".to_string())),
        "2026-06-01",
    )];
    store.append(Table::Schools, &school)?;

    let mut coach = CanonicalCoach::new(
        &school_id,
        "Coach Carter",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("carter@ada.test".to_string());
    coach.evidence = vec![Evidence::parsed(
        SourceRef::new(
            "coach_contacts_csv",
            Some("https://contacts.test/ada".to_string()),
        ),
        "2026-09-20",
    )];
    coach.tenure_evidence = vec![CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new("fixture", Some("https://contacts.test/ada".to_string())),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".to_string(),
        statement: "Synthetic academic-year appointment".to_string(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: coach.school.clone(),
            role: coach.role,
            program: CoachContactProgram::Team {
                sport: Sport::OutdoorTrack,
                gender: coach.gender,
            },
            mailbox: coach.professional_email.clone(),
        }),
    }];
    store.append(Table::Coaches, &coach)?;

    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "ada-runner"),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "synthetic_published_roster",
            Some("https://example.invalid/roster/ada-runner".to_string()),
        ),
    });
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.public_profile_urls = vec!["https://www.athletic.net/athlete/1".to_string()];
    athlete.evidence = vec![Evidence::parsed(
        SourceRef::new("wiaa_results", Some("https://wiaa.test/ada".to_string())),
        "2026-05-02",
    )];
    store.append(Table::Athletes, &athlete)?;
    Ok(())
}

fn publish(
    directory: &tempfile::TempDir,
) -> TestResult<(std::path::PathBuf, ExportDataset, Options)> {
    let store = Store::open(directory.path().join("store"))?;
    seed(&store)?;
    let options = Options::default();
    let published = crate::workbook::build(&store, &options)?;
    let generation = published
        .parent()
        .ok_or("missing generation directory")?
        .to_path_buf();
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    Ok((generation, dataset, options))
}

fn rewrite_cell(path: &std::path::Path, column: &str, value: &str) -> TestResult {
    let mut reader = csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    let index = header
        .iter()
        .position(|name| name == column)
        .ok_or_else(|| format!("missing published column {column}"))?;
    let mut rows: Vec<Vec<String>> = reader
        .records()
        .map(|record| record.map(|record| record.iter().map(str::to_owned).collect()))
        .collect::<Result<_, _>>()?;
    let row = rows.first_mut().ok_or("missing CSV data row to forge")?;
    let cell = row.get_mut(index).ok_or("missing CSV cell to forge")?;
    *cell = value.to_string();
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record(&header)?;
    for row in &rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}

#[test]
fn valid_generated_sidecars_pass_semantic_readback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    super::verify(&generation, &dataset, &options)?;
    Ok(())
}

#[test]
fn a_forged_census_sidecar_fails_semantic_readback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    let path = generation.join("census-core.json");
    let bytes = std::fs::read(&path)?;
    let mut report: serde_json::Value = serde_json::from_slice(&bytes)?;
    report["totals"]["athletes"] = serde_json::Value::from(9_999_u64);
    std::fs::write(&path, serde_json::to_vec(&report)?)?;
    let error = match super::verify(&generation, &dataset, &options) {
        Err(error) => error,
        Ok(()) => return Err("forged census accepted".into()),
    };
    check!(error.to_string().contains("census-core.json"), "{error}");
    Ok(())
}

#[test]
fn a_forged_recruiting_sidecar_fails_semantic_readback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    rewrite_cell(
        &generation.join("recruiting.csv"),
        "name",
        "Mallory Impostor",
    )?;
    let error = match super::verify(&generation, &dataset, &options) {
        Err(error) => error,
        Ok(()) => return Err("forged recruiting row accepted".into()),
    };
    check!(error.to_string().contains("recruiting.csv"), "{error}");
    Ok(())
}

#[test]
fn extra_and_missing_bundle_sidecars_are_rejected() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    let jsonl = generation.join("best-results-co2027.jsonl");
    let mut text = std::fs::read_to_string(&jsonl)?;
    text.push_str("{\"forged\":true}\n");
    std::fs::write(&jsonl, text)?;
    check!(super::verify(&generation, &dataset, &options).is_err());

    std::fs::remove_file(&jsonl)?;
    check!(super::verify(&generation, &dataset, &options).is_err());

    std::fs::remove_file(generation.join("recruiting.csv"))?;
    check!(super::verify(&generation, &dataset, &options).is_err());
    Ok(())
}
