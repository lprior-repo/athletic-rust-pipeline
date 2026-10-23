use super::TestResult;
use calamine::{open_workbook, Reader, Xlsx};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[path = "end_to_end/fixture.rs"]
mod fixture;
#[path = "end_to_end/mutation.rs"]
mod mutation;
#[path = "end_to_end/overflow.rs"]
mod overflow;
#[path = "end_to_end/profile_alias.rs"]
mod profile_alias;
#[path = "end_to_end/profile_prs.rs"]
mod profile_prs;
#[path = "end_to_end/profile_scope.rs"]
mod profile_scope;
#[path = "end_to_end/support.rs"]
mod support;

use fixture::IncludedFixture;
use mutation::{corrupt_cell, WorkbookMutation};

#[test]
fn include_publication_writes_only_reported_rows_from_reopened_retained_facts() -> TestResult {
    let fixture = IncludedFixture::new()?;
    let derivation = crate::report::Derivation::for_publication(
        &fixture.dataset,
        crate::report::Scope::AllSources,
        Some(2027),
        &fixture.options.jurisdictions,
    )?;
    check!(eq; derivation.athletes().len(), 0);
    check!(eq; derivation.performances().len(), 0);

    let censuses = crate::workbook::Censuses::of(
        &fixture.dataset,
        &fixture.store.out_dir(),
        &fixture.options.jurisdictions,
    )?;
    let workbook = crate::workbook::build_from(
        &fixture.dataset,
        &fixture.store,
        &fixture.options,
        &censuses,
    )?;
    let generation = workbook.parent().ok_or("publication generation")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    let archive_digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("frozen archive commitment")?;
    let reopened = crate::export::ExportDataset::reopen_frozen_with_digest(
        &generation.join("frozen-input.json"),
        archive_digest,
    )?;
    crate::workbook::verify::verify_frozen(&workbook, &reopened, &fixture.options)?;
    let publication = crate::workbook::publication::verify_published(&workbook)?;
    check!(!publication.generation_digest.is_empty());
    let original_bytes = std::fs::read(&workbook)?;
    let request = crate::export::ExportJobRequest::SelectedPublicationV1 {
        selection: census_domain::model::ProfileSelectionV1::new(
            fixture.store.store_identity()?,
            fixture
                .dataset
                .selected_profile_id()
                .ok_or("selected profile")?
                .clone(),
        ),
    };
    let retry = crate::export::ExportDataset::for_job_request(
        &fixture.store,
        "reported-include-publication-v1",
        &request,
    )?;
    let repeated =
        crate::workbook::build_from(&retry, &fixture.store, &fixture.options, &censuses)?;
    check!(eq; repeated, workbook);
    check!(eq; std::fs::read(&repeated)?, original_bytes);

    check!(eq; manifest.get("schema_revision").and_then(serde_json::Value::as_u64), Some(12));
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized reported row counts")?;
    check!(eq; counts.get("reported_performances.csv").and_then(serde_json::Value::as_u64), Some(3));
    check!(eq; counts.get("athletes.csv").and_then(serde_json::Value::as_u64), Some(0));
    check!(eq; counts.get("best_marks.csv").and_then(serde_json::Value::as_u64), Some(0));
    let reported_csv = std::fs::read(generation.join("reported_performances.csv"))?;
    let reported_csv_hash = format!("{:x}", Sha256::digest(&reported_csv));
    check!(eq;
        manifest.pointer("/artifacts/reported_performances.csv/sha256").and_then(serde_json::Value::as_str),
        Some(reported_csv_hash.as_str()),
    );
    check!(eq;
        manifest.pointer("/artifacts/reported_performances.csv/bytes").and_then(serde_json::Value::as_u64),
        Some(u64::try_from(reported_csv.len())?),
    );
    let csv_count = csv::Reader::from_path(generation.join("reported_performances.csv"))?
        .records()
        .try_fold(0_u64, |count, row| -> TestResult<u64> {
            row?;
            count
                .checked_add(1)
                .ok_or_else(|| "reported CSV row count overflow".into())
        })?;
    check!(eq; csv_count, 3);
    let mut sidecar = csv::Reader::from_path(generation.join("reported_performances.csv"))?;
    let headers = sidecar.headers()?.clone();
    let sidecar_performance = headers
        .iter()
        .position(|header| header == "Performance ID")
        .ok_or("reported performance ID column")?;
    let sidecar_mark = headers
        .iter()
        .position(|header| header == "Mark")
        .ok_or("reported mark column")?;
    let retained = sidecar
        .records()
        .map(|record| {
            let record = record?;
            let id = record
                .get(sidecar_performance)
                .ok_or("reported performance ID")?
                .to_owned();
            let mark = record.get(sidecar_mark).ok_or("reported mark")?.to_owned();
            Ok((id, mark))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    check!(eq; retained.len(), 3);
    check!(eq; retained.get(fixture.first_performance.id.as_str()).map(String::as_str), Some("50.00"));
    check!(eq; retained.get(fixture.faster_performance.id.as_str()).map(String::as_str), Some("49.00"));
    check!(eq; retained.get(fixture.second_performance.id.as_str()).map(String::as_str), Some("55.00"));

    let mut reader: Xlsx<_> = open_workbook(&workbook)?;
    check!(eq; reader.sheet_names(), vec!["Athletes".to_owned()]);
    let athletes = reader.worksheet_range("Athletes")?;
    let rows = athletes.rows().collect::<Vec<_>>();
    check!(eq; rows.len(), 3);
    let headers = rows.first().ok_or("single worksheet headers")?;
    let column = |name: &str| -> TestResult<usize> {
        headers
            .iter()
            .position(|cell| cell.to_string() == name)
            .ok_or_else(|| format!("missing worksheet column {name}").into())
    };
    for name in [
        "School Address",
        "Observed Grade",
        "Grade School Year",
        "Head TF Coach",
        "Head XC Coach",
        "Source-declared PRs",
    ] {
        column(name)?;
    }
    check!(headers
        .iter()
        .all(|cell| !cell.to_string().to_ascii_lowercase().contains("email")));
    let id_column = column("Athlete ID")?;
    let by_id = rows
        .iter()
        .skip(1)
        .map(|row| {
            Ok((
                row.get(id_column)
                    .ok_or("worksheet athlete ID")?
                    .to_string(),
                *row,
            ))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    check!(eq; by_id.len(), 2);
    for athlete in [&fixture.first, &fixture.second] {
        let row = by_id
            .get(athlete.id.as_str())
            .ok_or("source reported athlete")?;
        check!(eq; row.get(column("Record Basis")?).map(ToString::to_string).as_deref(), Some("source_reported"));
        check!(eq; row.get(column("Verification Status")?).map(ToString::to_string).as_deref(), Some("reported_unverified_needs_review"));
        check!(eq; row.get(column("School")?).map(ToString::to_string).as_deref(), Some("Synthetic Michigan High"));
        check!(eq; row.get(column("School State")?).map(ToString::to_string).as_deref(), Some("MI"));
        check!(eq; row.get(column("Current school")?).map(ToString::to_string).as_deref(), Some("Unknown"));
        check!(row
            .get(column("Observed bests")?)
            .is_none_or(|cell| cell.to_string().is_empty()));
        check!(row
            .get(column("Source-declared PRs")?)
            .is_none_or(|cell| cell.to_string().is_empty()));
    }
    let first = by_id
        .get(fixture.first.id.as_str())
        .ok_or("first athlete")?;
    let best = first
        .get(column("Reported bests")?)
        .ok_or("all first athlete bests")?
        .to_string();
    for literal in [
        "Calculated PR: 49.00",
        "PR date: 2026-05-11",
        "Meet: Second 400m Meet",
        "Timing Class: fat",
        "Surface: outdoor",
        "https://mi.milesplit.com/results/reported-49",
        fixture.faster_performance.id.as_str(),
    ] {
        check!(best.contains(literal));
    }
    check!(!best.contains("Calculated PR: 50.00"));
    let second = by_id
        .get(fixture.second.id.as_str())
        .ok_or("second athlete")?;
    let second_best = second
        .get(column("Reported bests")?)
        .ok_or("second athlete best")?
        .to_string();
    check!(second_best.contains("Calculated PR: 55.00"));
    check!(second_best.contains(fixture.second_performance.id.as_str()));
    let mutations = [
        WorkbookMutation {
            sheet: "Athletes",
            header: "Reported bests",
            row_key: fixture.first.id.as_str(),
            value: "Calculated PR: 48.25",
        },
        WorkbookMutation {
            sheet: "Athletes",
            header: "School State",
            row_key: fixture.first.id.as_str(),
            value: "",
        },
        WorkbookMutation {
            sheet: "Athletes",
            header: "School",
            row_key: fixture.second.id.as_str(),
            value: "Wrong School",
        },
        WorkbookMutation {
            sheet: "Athletes",
            header: "Current school",
            row_key: fixture.first.id.as_str(),
            value: "Unsupported current school",
        },
        WorkbookMutation {
            sheet: "Athletes",
            header: "Observed Grade",
            row_key: fixture.first.id.as_str(),
            value: "12",
        },
        WorkbookMutation {
            sheet: "Athletes",
            header: "Source-declared PRs",
            row_key: fixture.first.id.as_str(),
            value: "Invented profile PR",
        },
    ];
    for mutation in mutations {
        let tampered = fixture.directory.path().join("tampered.xlsx");
        corrupt_cell(&workbook, &tampered, mutation)?;
        check!(
            crate::workbook::verify::verify_frozen(&tampered, &reopened, &fixture.options).is_err()
        );
    }
    for extra in [true, false] {
        let tampered = fixture.directory.path().join("changed-inventory.xlsx");
        mutation::change_inventory(&workbook, &tampered, extra)?;
        check!(
            crate::workbook::verify::verify_frozen(&tampered, &reopened, &fixture.options).is_err()
        );
    }
    std::fs::remove_file(generation.join("reported_performances.csv"))?;
    check!(crate::workbook::publication::verify_published(&workbook).is_err());
    Ok(())
}
