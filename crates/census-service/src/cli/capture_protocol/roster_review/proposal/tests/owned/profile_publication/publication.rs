use super::*;
use std::path::Path;

pub(super) fn verify(
    fixture: fixture::Fixture,
    store: Store,
    dataset: ExportDataset,
    request: &ExportJobRequest,
    digest: &str,
) -> TestResult {
    let options = workbook::Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: Some(
            fixture
                .path
                .parent()
                .ok_or("fixture parent")?
                .join("publication"),
        ),
        limit: None,
        scope: census_report::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        jurisdictions: vec![UsJurisdiction::Michigan],
    };
    let censuses = workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    let path = workbook::build_from(&dataset, &store, &options, &censuses)?;
    let repeated = ExportDataset::for_job_request(&store, "owned-profile-publication", request)?;
    check!(eq; workbook::build_from(&repeated, &store, &options, &censuses)?, path);
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    let directory = path.parent().ok_or("generation")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest["schema_revision"].as_u64(), Some(14));
    check!(eq; manifest.pointer("/normalized_csv_rows/source_profile_prs.csv").and_then(serde_json::Value::as_u64), Some(1));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(serde_json::Value::as_u64), Some(0));
    let commitment = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("frozen digest")?;
    let csv = std::fs::read_to_string(directory.join("source_profile_prs.csv"))?;
    for marker in [
        "4:30.00",
        "2025-03-01",
        "1600 Meter Run",
        "2026-10-02T12:00:10",
        "source_declared_pr_unverified",
    ] {
        check!(csv.contains(marker));
    }
    drop((dataset, repeated, store));
    let offline = fixture.path.with_extension("offline");
    std::fs::rename(&fixture.path, &offline)?;
    check!(!fixture.path.exists());
    let frozen =
        ExportDataset::reopen_frozen_with_digest(&directory.join("frozen-input.json"), commitment)?;
    workbook::verify::verify_frozen(&path, &frozen, &options)?;
    workbook::publication::verify_published(&path)?;
    rejects_mutation(directory, &path, &csv)?;
    rejects_source_change(directory, commitment, digest)?;
    Ok(())
}

fn rejects_source_change(directory: &Path, commitment: &str, digest: &str) -> TestResult {
    let archive = directory.join("frozen-input.json");
    let object = directory.join("frozen-input.json.proofs").join(digest);
    let original = std::fs::read(&object)?;
    std::fs::write(&object, b"controlled changed owned source")?;
    check!(ExportDataset::reopen_frozen_with_digest(&archive, commitment).is_err());
    std::fs::write(&object, &original)?;
    ExportDataset::reopen_frozen_with_digest(&archive, commitment)?;
    std::fs::remove_file(&object)?;
    check!(ExportDataset::reopen_frozen_with_digest(&archive, commitment).is_err());
    std::fs::write(&object, original)?;
    ExportDataset::reopen_frozen_with_digest(&archive, commitment)?;
    Ok(())
}

fn rejects_mutation(directory: &Path, workbook_path: &Path, original: &str) -> TestResult {
    let csv = directory.join("source_profile_prs.csv");
    let changed = original.replacen("4:30.00", "9:59.99", 1);
    check!(ne; changed, original);
    std::fs::write(&csv, changed)?;
    check!(workbook::publication::verify_published(workbook_path).is_err());
    std::fs::write(&csv, original)?;
    workbook::publication::verify_published(workbook_path)?;
    Ok(())
}
