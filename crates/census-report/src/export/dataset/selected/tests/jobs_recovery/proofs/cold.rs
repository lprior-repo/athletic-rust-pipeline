use super::*;
use crate::export::ReportedProfilePrTable;
use crate::report::Scope;
use crate::workbook::publication;
use calamine::{open_workbook, DataType, Reader, Xlsx};
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::{CanonicalAthlete, GradYear};
use std::collections::BTreeSet;

const SOURCE_PR_HEADER: &[&str] = &[
    "canonical_athlete_id",
    "source_namespace",
    "source_athlete_id",
    "event",
    "source_declared_pr",
    "source_mode",
    "source_level",
    "published_date",
    "source_profile_url",
    "capture_acquired_at",
    "capture_body_sha256",
    "retained_root",
    "retained_payload_sha256",
    "retained_payload_bytes",
    "interpretation",
    "row_ordinal",
    "byte_offset",
    "byte_length",
    "record_basis",
    "verification_status",
];

pub(super) fn verify(recovered: Recovered) -> TestResult {
    let Recovered {
        fixture,
        dataset,
        root: source_job,
        initial,
        orphan,
        ..
    } = recovered;
    let publication_directory = super::super::super::temporary()?;
    let root = publication_directory.path().join("publication");
    let expected = super::super::super::witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let path = super::super::super::publication::publish(&fixture, &dataset, &root)?;
    let verified = publication::verify_published(&path)?;
    check!(verified.workbook.mapped_athletes > 0);
    check!(eq; publication::current_workbook(&root)?, path);
    let claims = profile_readback(&path, &dataset)?;
    let generation = path.parent().ok_or("generation directory")?.to_path_buf();
    check!(generation.join("current_affiliations.csv").is_file());
    check!(generation.join("participation_exclusions.csv").is_file());
    let objects = generation.join("frozen-input.json.proofs");
    check!(objects.is_dir());
    preserve(&objects, &initial)?;
    let original = initial.get(&orphan).ok_or("retained orphan")?.clone();
    let object = objects.join(&orphan);
    let protected_before = protected(&root, &path)?;
    let store_root = fixture.store.root().to_path_buf();
    drop(dataset);
    fixture.remove_sources()?;
    check!(!store_root.exists());
    check!(!source_job.exists());
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; reopened.generation_digest, verified.generation_digest);
    check!(eq; super::super::super::witness::capture(&reopened.dataset)?, expected);
    check!(eq; profile_readback(&path, &reopened.dataset)?, claims);
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    drop(reopened);
    tamper(&path, &object, &original)?;
    check!(eq; protected(&root, &path)?, protected_before);
    std::fs::write(&object, &original)?;
    publication::verify_published(&path)?;
    missing(&path, &object)?;
    check!(eq; protected(&root, &path)?, protected_before);
    std::fs::write(&object, &original)?;
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    Ok(())
}

fn protected(root: &Path, workbook: &Path) -> TestResult<(PathBuf, Vec<u8>, Vec<u8>)> {
    let generation = workbook.parent().ok_or("generation directory")?;
    Ok((
        std::fs::read_link(root.join("current"))?,
        std::fs::read(generation.join("manifest.json"))?,
        std::fs::read(workbook)?,
    ))
}

fn tamper(workbook: &Path, object: &Path, original: &[u8]) -> TestResult {
    let mut changed = original.to_vec();
    *changed.first_mut().ok_or("nonempty orphan")? ^= 1;
    check!(eq; changed.len(), original.len());
    std::fs::write(object, &changed)?;
    let error = publication::verify_published(workbook)
        .err()
        .ok_or("tampered orphan accepted")?;
    check!(
        matches!(error, ReportError::ArtifactCorrupt { path, detail }
        if path == object && detail == "the frozen object does not hash to its name")
    );
    check!(eq; std::fs::read(object)?, changed);
    Ok(())
}

fn missing(workbook: &Path, object: &Path) -> TestResult {
    std::fs::remove_file(object)?;
    let error = publication::verify_published(workbook)
        .err()
        .ok_or("missing orphan accepted")?;
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == "selected proof directory differs from its authenticated object inventory"));
    check!(!object.exists());
    Ok(())
}

fn profile_readback(
    workbook: &Path,
    dataset: &ExportDataset,
) -> TestResult<(String, Vec<Vec<String>>)> {
    check!(eq; dataset.athletes.len(), 1);
    let athlete = dataset.athletes.first().ok_or("source PR athlete")?;
    let (expected_cell, mut expected_records) = profile_expectations(dataset, athlete)?;
    let public_id = dataset
        .verified_public_identities()?
        .public_id(athlete.id.as_str())
        .ok_or("accepted source PR public athlete ID")?;
    let actual_cell = profile_cell(workbook, public_id.as_str())?;
    check!(eq; actual_cell, expected_cell);
    check!(eq; actual_cell.lines().count(), 94);
    check!(eq; actual_cell.lines().filter(|line| line.starts_with("Source-declared PR capture (unverified);")).count(), 2);
    let generation = workbook.parent().ok_or("source PR generation")?;
    let mut reader = csv::Reader::from_path(generation.join("source_profile_prs.csv"))?;
    check!(eq; reader.headers()?.iter().collect::<Vec<_>>(), SOURCE_PR_HEADER.to_vec());
    let mut actual_records = reader
        .records()
        .map(|record| record.map(|record| record.iter().map(str::to_owned).collect::<Vec<_>>()))
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; actual_records.len(), 92);
    expected_records.sort_unstable();
    actual_records.sort_unstable();
    check!(eq; actual_records, expected_records);
    Ok((actual_cell, actual_records))
}

fn profile_expectations(
    dataset: &ExportDataset,
    athlete: &CanonicalAthlete,
) -> TestResult<(String, Vec<Vec<String>>)> {
    let tables = dataset
        .reported_profile_prs_for(athlete)
        .collect::<Vec<_>>();
    check!(eq; tables.len(), 2);
    let first = tables.first().ok_or("first captured PR table")?;
    let second = tables.get(1).ok_or("second captured PR table")?;
    check!(ne; first.root.key, second.root.key);
    check!(eq; first.rows.as_slice(), second.rows.as_slice());
    let mut detailed = BTreeSet::new();
    let mut grouped = Vec::new();
    let mut records = Vec::new();
    for table in tables {
        check!(eq; table.rows.len(), 46);
        grouped.push(format!(
            "Source-declared PR capture (unverified); Profile: {}; Capture: {}; Root: {}; Body interpretation: {}",
            table.profile_url, table.acquired_at.to_rfc3339(), table.root.key, table.interpretation,
        ));
        for row in &table.rows {
            detailed.insert(detailed_literal(table, row));
            grouped.push(grouped_literal(row));
            records.push(profile_record(athlete.id.as_str(), table, row)?);
        }
    }
    let detailed = detailed.into_iter().collect::<Vec<_>>().join("\n");
    let grouped = grouped.join("\n");
    check!(detailed.encode_utf16().count() > 32_767);
    check!(grouped.encode_utf16().count() <= 32_767);
    check!(eq; records.len(), 92);
    Ok((grouped, records))
}

fn profile_cell(workbook: &Path, athlete: &str) -> TestResult<String> {
    let mut book: Xlsx<_> = open_workbook(workbook)?;
    let sheet = book.worksheet_range("Athletes")?;
    let headers = sheet.rows().next().ok_or("Athletes headers")?;
    let id = headers
        .iter()
        .position(|cell| cell.get_string().is_some_and(|text| text == "Athlete ID"))
        .ok_or("Athlete ID column")?;
    let claims = headers
        .iter()
        .position(|cell| {
            cell.get_string()
                .is_some_and(|text| text == "Source-declared PRs")
        })
        .ok_or("Source-declared PRs column")?;
    let mut rows = sheet.rows().skip(1).filter(|row| {
        row.get(id)
            .is_some_and(|value| value.get_string().is_some_and(|text| text == athlete))
    });
    let row = rows.next().ok_or("source PR athlete worksheet row")?;
    check!(rows.next().is_none());
    Ok(row
        .get(claims)
        .ok_or("source PR worksheet cell")?
        .to_string())
}

fn detailed_literal(table: &ReportedProfilePrTable, row: &SourceProfilePrV1) -> String {
    let locator = row.locator();
    format!(
        "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Profile: {}; Capture: {}; Root: {}; Body interpretation: {}; Locator: {}:{}:{}",
        row.event(), row.published_mark(), row.source_mode(), row.source_level(),
        row.published_date(), table.profile_url, table.acquired_at.to_rfc3339(),
        table.root.key, table.interpretation, locator.ordinal(),
        locator.byte_offset(), locator.byte_length(),
    )
}

fn grouped_literal(row: &SourceProfilePrV1) -> String {
    let locator = row.locator();
    format!(
        "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Locator: {}:{}:{}",
        row.event(),
        row.published_mark(),
        row.source_mode(),
        row.source_level(),
        row.published_date(),
        locator.ordinal(),
        locator.byte_offset(),
        locator.byte_length(),
    )
}

fn profile_record(
    athlete: &str,
    table: &ReportedProfilePrTable,
    row: &SourceProfilePrV1,
) -> TestResult<Vec<String>> {
    let locator = row.locator();
    Ok(vec![
        athlete.to_owned(),
        serde_json::to_string(&table.source.namespace)?,
        table.source.id.clone(),
        row.event().to_owned(),
        row.published_mark().to_owned(),
        row.source_mode().to_owned(),
        row.source_level().to_owned(),
        row.published_date().to_owned(),
        table.profile_url.clone(),
        table.acquired_at.to_rfc3339(),
        table.body_sha256.clone(),
        serde_json::to_string(&table.root)?,
        table.root.payload.digest.as_str().to_owned(),
        table.root.payload.bytes.to_string(),
        table.interpretation.clone(),
        locator.ordinal().to_string(),
        locator.byte_offset().to_string(),
        locator.byte_length().to_string(),
        "source_reported".to_owned(),
        "source_declared_pr_unverified".to_owned(),
    ])
}
