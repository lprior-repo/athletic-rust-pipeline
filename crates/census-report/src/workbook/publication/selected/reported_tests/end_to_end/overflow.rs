use super::fixture::IncludedFixture;
use super::mutation::{corrupt_cell, WorkbookMutation};
use super::TestResult;
use calamine::{open_workbook, DataType, Reader, Xlsx};

#[test]
fn aggregate_evidence_overflow_preserves_full_frozen_and_csv_evidence_without_marker() -> TestResult
{
    let athlete_note = format!("athlete-{}", "a".repeat(14_000));
    let performance_note = format!("performance-{}", "b".repeat(14_000));
    let fixture = IncludedFixture::with_aggregate_evidence(&athlete_note, &performance_note)?;
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
    let digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("frozen archive digest")?;
    let reopened = crate::export::ExportDataset::reopen_frozen_with_digest(
        &generation.join("frozen-input.json"),
        digest,
    )?;
    let athlete = reopened
        .athletes
        .iter()
        .find(|athlete| athlete.id == fixture.first.id)
        .ok_or("reopened athlete")?;
    let performance = reopened
        .performances
        .iter()
        .find(|performance| performance.id == fixture.faster_performance.id)
        .ok_or("reopened performance")?;
    check!(eq; athlete.evidence.first().and_then(|evidence| evidence.note.as_deref()), Some(athlete_note.as_str()));
    check!(athlete.evidence.iter().all(|evidence| evidence
        .note
        .as_ref()
        .is_none_or(|note| note.len() <= 16 * 1024)));
    check!(athlete
        .evidence
        .iter()
        .any(|evidence| evidence.note.as_deref()
            == Some(format!("{athlete_note}-additional").as_str())));
    check!(eq; performance.evidence.first().and_then(|evidence| evidence.note.as_deref()), Some(performance_note.as_str()));
    let athlete_json = serde_json::to_string(&athlete.evidence)?;
    let performance_json = serde_json::to_string(&performance.evidence)?;
    check!(athlete_json.encode_utf16().count() < 32_767);
    check!(performance_json.encode_utf16().count() < 32_767);
    let evidence = athlete
        .evidence
        .iter()
        .chain(
            reopened
                .performances
                .iter()
                .filter(|performance| performance.athlete == athlete.id)
                .flat_map(|performance| performance.evidence.iter()),
        )
        .collect::<Vec<_>>();
    let full = serde_json::to_string(&evidence)?;
    let units = full.encode_utf16().count();
    check!(units > 32_767);
    let mut reader: Xlsx<_> = open_workbook(&workbook)?;
    check!(eq; reader.sheet_names(), vec!["Athletes"]);
    let athletes = reader.worksheet_range("Athletes")?;
    let headers = athletes.rows().next().ok_or("athlete header")?;
    let id = headers
        .iter()
        .position(|cell| cell.get_string().is_some_and(|text| text == "Athlete ID"))
        .ok_or("athlete ID column")?;
    let name = headers
        .iter()
        .position(|cell| cell.get_string().is_some_and(|text| text == "Name"))
        .ok_or("athlete name column")?;
    let basis = headers
        .iter()
        .position(|cell| cell.get_string().is_some_and(|text| text == "Record Basis"))
        .ok_or("record basis column")?;
    let status = headers
        .iter()
        .position(|cell| {
            cell.get_string()
                .is_some_and(|text| text == "Verification Status")
        })
        .ok_or("verification status column")?;
    let reported = headers
        .iter()
        .position(|cell| {
            cell.get_string()
                .is_some_and(|text| text == "Reported bests")
        })
        .ok_or("reported bests column")?;
    check!(headers.iter().all(|cell| !cell
        .get_string()
        .is_some_and(|text| text == "Reported Source Evidence")));
    let row = athletes
        .rows()
        .skip(1)
        .find(|row| {
            row.get(id).is_some_and(|cell| {
                cell.get_string()
                    .is_some_and(|text| text == athlete.id.as_str())
            })
        })
        .ok_or("reported athlete row")?;
    check!(eq; row.get(name).ok_or("name cell")?.to_string(), athlete.canonical_name);
    check!(eq; row.get(basis).ok_or("basis cell")?.to_string(), "source_reported");
    check!(eq; row.get(status).ok_or("status cell")?.to_string(), "reported_unverified_needs_review");
    let reported_text = row.get(reported).ok_or("reported bests cell")?.to_string();
    check!(reported_text.contains(&performance.id.to_string()));
    let mut csv = csv::Reader::from_path(generation.join("reported_performances.csv"))?;
    let headers = csv.headers()?.clone();
    let id = headers
        .iter()
        .position(|header| header == "Performance ID")
        .ok_or("CSV performance ID")?;
    let evidence = headers
        .iter()
        .position(|header| header == "Performance Evidence")
        .ok_or("CSV performance evidence")?;
    let mut found = false;
    for record in csv.records() {
        let record = record?;
        if record.get(id) == Some(performance.id.as_str()) {
            check!(eq; record.get(evidence), Some(performance_json.as_str()));
            found = true;
        }
    }
    check!(found);
    crate::workbook::verify::verify_frozen(&workbook, &reopened, &fixture.options)?;
    crate::workbook::publication::verify_published(&workbook)?;
    for value in ["", "[]", "different performance id", "changed text"] {
        let tampered = fixture.directory.path().join("tampered-overflow.xlsx");
        corrupt_cell(
            &workbook,
            &tampered,
            WorkbookMutation {
                sheet: "Athletes",
                header: "Reported bests",
                row_key: athlete.id.as_str(),
                value,
            },
        )?;
        check!(
            crate::workbook::verify::verify_frozen(&tampered, &reopened, &fixture.options).is_err()
        );
    }
    Ok(())
}
