use super::TestResult;
use calamine::{open_workbook, Data, Reader, Xlsx};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[path = "end_to_end/fixture.rs"]
mod fixture;
#[path = "end_to_end/mutation.rs"]
mod mutation;
#[path = "end_to_end/overflow.rs"]
mod overflow;
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

    check!(eq; manifest.get("schema_revision").and_then(serde_json::Value::as_u64), Some(9));
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized revision-nine row counts")?;
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
    let athletes = reader.worksheet_range("Athletes")?;
    let prs = reader.worksheet_range("PRs")?;
    let athlete_rows = athletes.rows().collect::<Vec<_>>();
    let pr_rows = prs.rows().collect::<Vec<_>>();
    check!(eq; athlete_rows.len(), 3);
    check!(eq; pr_rows.len(), 3);
    let athlete_header = athlete_rows.first().ok_or("Athletes header")?;
    let pr_header = pr_rows.first().ok_or("PRs header")?;
    let column = |header: &[Data], name: &str| -> TestResult<usize> {
        header
            .iter()
            .position(|cell| cell.to_string() == name)
            .ok_or_else(|| format!("missing workbook column {name}").into())
    };
    let athlete_basis = column(athlete_header, "Record Basis")?;
    let athlete_status = column(athlete_header, "Verification Status")?;
    let athlete_id = column(athlete_header, "Athlete ID")?;
    let athlete_evidence = column(athlete_header, "Reported Source Evidence")?;
    let athlete_school_name = column(athlete_header, "Reported School Name")?;
    let athlete_state = column(athlete_header, "Reported State")?;
    let athlete_rows_by_id = athlete_rows
        .iter()
        .skip(1)
        .map(|row| {
            let id = row
                .get(athlete_id)
                .map(ToString::to_string)
                .ok_or("reported athlete ID")?;
            Ok((id, *row))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    check!(eq; athlete_rows_by_id.len(), 2);
    for athlete in [&fixture.first, &fixture.second] {
        let row = athlete_rows_by_id
            .get(athlete.id.as_str())
            .ok_or("expected source-reported athlete row")?;
        check!(eq; row.get(athlete_basis).map(ToString::to_string).as_deref(), Some("source_reported"));
        check!(eq; row.get(athlete_status).map(ToString::to_string).as_deref(), Some("reported_unverified_needs_review"));
        check!(row
            .get(athlete_evidence)
            .is_some_and(|cell| !cell.to_string().is_empty()));
        check!(eq; row.get(athlete_school_name).map(ToString::to_string).as_deref(), Some("Synthetic Michigan High"));
        check!(eq; row.get(athlete_state).map(ToString::to_string).as_deref(), Some("MI"));
    }

    let pr_basis = column(pr_header, "Record Basis")?;
    let pr_status = column(pr_header, "Verification Status")?;
    let pr_athlete_id = column(pr_header, "Athlete ID")?;
    let pr_mark = column(pr_header, "Calculated PR")?;
    let pr_performance_id = column(pr_header, "Performance ID")?;
    let pr_evidence = column(pr_header, "Performance Evidence")?;
    let mut reported_marks = BTreeMap::new();
    for row in pr_rows.iter().skip(1) {
        check!(eq; row.get(pr_basis).map(ToString::to_string).as_deref(), Some("source_reported"));
        check!(eq; row.get(pr_status).map(ToString::to_string).as_deref(), Some("reported_best_in_retained_set"));
        check!(row
            .get(pr_evidence)
            .is_some_and(|cell| !cell.to_string().is_empty()));
        let performance_id = row
            .get(pr_performance_id)
            .map(ToString::to_string)
            .ok_or("reported performance ID")?;
        let mark = row
            .get(pr_mark)
            .map(ToString::to_string)
            .ok_or("reported PR mark")?;
        let athlete = row
            .get(pr_athlete_id)
            .map(ToString::to_string)
            .ok_or("reported PR athlete ID")?;
        reported_marks.insert(mark, (performance_id, athlete, *row));
    }
    check!(eq; reported_marks.len(), 2);
    check!(reported_marks.contains_key("49.00"));
    check!(reported_marks.contains_key("55.00"));
    check!(eq;
        reported_marks.get("49.00").map(|(_, athlete, _)| athlete.as_str()),
        Some(fixture.first.id.as_str()),
    );
    check!(eq;
        reported_marks.get("55.00").map(|(_, athlete, _)| athlete.as_str()),
        Some(fixture.second.id.as_str()),
    );
    check!(eq;
        reported_marks.get("49.00").map(|(id, _, _)| id.as_str()),
        Some(fixture.faster_performance.id.as_str()),
    );
    check!(eq;
        reported_marks.get("55.00").map(|(id, _, _)| id.as_str()),
        Some(fixture.second_performance.id.as_str()),
    );
    if let Some(current_school) = pr_header
        .iter()
        .position(|cell| cell.to_string() == "Current school")
    {
        for (_, _, row) in reported_marks.values() {
            check!(row
                .get(current_school)
                .is_none_or(|cell| cell.to_string().is_empty()));
        }
    }

    let mutations = [
        WorkbookMutation {
            sheet: "PRs",
            header: "Calculated PR",
            row_key: fixture.faster_performance.id.as_str(),
            value: "48.25",
        },
        WorkbookMutation {
            sheet: "Athletes",
            header: "Reported State",
            row_key: fixture.first.id.as_str(),
            value: "",
        },
        WorkbookMutation {
            sheet: "Athletes",
            header: "Reported School Name",
            row_key: fixture.second.id.as_str(),
            value: "Wrong School",
        },
        WorkbookMutation {
            sheet: "PRs",
            header: "Performance Evidence",
            row_key: fixture.second_performance.id.as_str(),
            value: "[]",
        },
    ];
    for mutation in mutations {
        let tampered = fixture.directory.path().join("tampered.xlsx");
        corrupt_cell(&workbook, &tampered, mutation)?;
        check!(
            crate::workbook::verify::verify_frozen(&tampered, &reopened, &fixture.options).is_err()
        );
    }
    std::fs::remove_file(generation.join("reported_performances.csv"))?;
    check!(crate::workbook::publication::verify_published(&workbook).is_err());
    Ok(())
}
