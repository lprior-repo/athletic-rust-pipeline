use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, CoachTenure, CoachTenureEvidence,
    Evidence, Gender, GradYear, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use crate::export::ExportDataset;
use crate::workbook::Options;

mod bests;

fn seed(store: &Store) {
    let (mut school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Ada High", "ada high");
    school.city = Some("Madison".to_string());
    school.athletics_website = Some("https://ada.test/athletics".to_string());
    school.evidence = vec![Evidence::parsed(
        SourceRef::new("wiaa_results", Some("https://wiaa.test/ada".to_string())),
        "2026-06-01",
    )];
    store
        .append(Table::Schools, &school)
        .expect("school appends");

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
            school_year: SchoolYear::new(2026).expect("2026 is a season"),
        },
        source: SourceRef::new("fixture", Some("https://contacts.test/ada".to_string())),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".to_string(),
        statement: "Synthetic academic-year appointment".to_string(),
    }];
    store.append(Table::Coaches, &coach).expect("coach appends");

    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "ada-runner"),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.public_profile_urls = vec!["https://www.athletic.net/athlete/1".to_string()];
    athlete.evidence = vec![Evidence::parsed(
        SourceRef::new("wiaa_results", Some("https://wiaa.test/ada".to_string())),
        "2026-05-02",
    )];
    store
        .append(Table::Athletes, &athlete)
        .expect("athlete appends");
}

fn publish(directory: &tempfile::TempDir) -> (std::path::PathBuf, ExportDataset, Options) {
    let store = Store::open(directory.path().join("store")).expect("own store");
    seed(&store);
    let options = Options::default();
    let published = crate::workbook::build(&store, &options).expect("publish generation");
    let generation = published
        .parent()
        .expect("generation directory")
        .to_path_buf();
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))
        .expect("reopen frozen input");
    (generation, dataset, options)
}

fn rewrite_cell(path: &std::path::Path, column: &str, value: &str) {
    let mut reader = csv::Reader::from_path(path).expect("published CSV reads");
    let header = reader.headers().expect("header row").clone();
    let index = header
        .iter()
        .position(|name| name == column)
        .expect("published column");
    let mut rows: Vec<Vec<String>> = reader
        .records()
        .map(|record| {
            record
                .expect("data row")
                .iter()
                .map(str::to_owned)
                .collect()
        })
        .collect();
    if let Some(row) = rows.first_mut() {
        if let Some(cell) = row.get_mut(index) {
            *cell = value.to_string();
        }
    }
    let mut writer = csv::Writer::from_path(path).expect("published CSV writes");
    writer.write_record(&header).expect("header writes");
    for row in &rows {
        writer.write_record(row).expect("row writes");
    }
    writer.flush().expect("published CSV flushes");
}

#[test]
fn valid_generated_sidecars_pass_semantic_readback() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let (generation, dataset, options) = publish(&directory);
    super::verify(&generation, &dataset, &options).expect("sidecars verify");
}

#[test]
fn a_forged_census_sidecar_fails_semantic_readback() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let (generation, dataset, options) = publish(&directory);
    let path = generation.join("census-core.json");
    let bytes = std::fs::read(&path).expect("census reads");
    let mut report: serde_json::Value = serde_json::from_slice(&bytes).expect("census decodes");
    report["totals"]["athletes"] = serde_json::Value::from(9_999_u64);
    std::fs::write(&path, serde_json::to_vec(&report).expect("census encodes"))
        .expect("census forges");
    let error = super::verify(&generation, &dataset, &options).expect_err("forged census refused");
    assert!(error.to_string().contains("census-core.json"), "{error}");
}

#[test]
fn a_forged_recruiting_sidecar_fails_semantic_readback() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let (generation, dataset, options) = publish(&directory);
    rewrite_cell(
        &generation.join("recruiting.csv"),
        "name",
        "Mallory Impostor",
    );
    let error =
        super::verify(&generation, &dataset, &options).expect_err("forged recruiting row refused");
    assert!(error.to_string().contains("recruiting.csv"), "{error}");
}

#[test]
fn extra_and_missing_bundle_sidecars_are_rejected() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let (generation, dataset, options) = publish(&directory);
    let jsonl = generation.join("best-results-co2027.jsonl");
    let mut text = std::fs::read_to_string(&jsonl).expect("best-results reads");
    text.push_str("{\"forged\":true}\n");
    std::fs::write(&jsonl, text).expect("extra record appended");
    assert!(super::verify(&generation, &dataset, &options).is_err());

    std::fs::remove_file(&jsonl).expect("best-results removed");
    assert!(super::verify(&generation, &dataset, &options).is_err());

    std::fs::remove_file(generation.join("recruiting.csv")).expect("recruiting removed");
    assert!(super::verify(&generation, &dataset, &options).is_err());
}
