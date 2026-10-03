use anyhow::{ensure, Context};
use calamine::{Data, Reader};
use census_domain::model::{CanonicalSchool, SchoolYear};
use census_report::export::postal::{POSTAL_CSV_HEADERS, POSTAL_HEADERS};
use census_report::export::ExportDataset;
use census_report::workbook::{self, Censuses};
use census_store::Store;
use serde_json::{json, Value};
use std::path::Path;

use super::artifacts::{write_json, Result};
use super::postal::verify_positions;
use super::{CAPTURED, MODEL, NAME, ORG};

pub(super) fn publish_and_readback(
    root: &Path,
    store: &Store,
    dataset: &ExportDataset,
    observed: &str,
    metadata: &[Value],
) -> Result<(Value, [String; 12])> {
    let school = dataset.schools.values().next().context("school absent")?;
    let options = workbook::Options {
        out: Some(root.join("publication")),
        school_year: Some(SchoolYear::new(2026).context("unsupported school year")?),
        ..Default::default()
    };
    let censuses = Censuses::of(dataset, &store.out_dir());
    let path = workbook::build_from(dataset, store, &options, &censuses)?;
    let verified = workbook::verify::verify_frozen(&path, dataset, &options)?;
    let publication = workbook::publication::verify_published(&path)?;
    ensure!(
        verified.mapped_athletes == 0 && publication.workbook.mapped_athletes == 0,
        "qualification unexpectedly includes mapped athletes"
    );
    ensure!(
        workbook::publication::current_workbook(
            options.out.as_deref().context("publication root absent")?
        )? == path,
        "publication pointer does not bind the rendered workbook"
    );
    dataset.ensure_current(store)?;
    let xlsx = read_xlsx(&path, school, CAPTURED)?;
    write_json(
        &root.join("xlsx-readback.json"),
        &serde_json::to_value(&xlsx)?,
    )?;
    let evidence = json!({"status": "awaiting_csv_cli", "qualification_only": true,
        "generator_model": MODEL, "model_calls": 0, "synthetic_athletes": 0, "public_athletes": 0,
        "capture_origin": "retained public fixtures, offline replay; not fresh network acquisition",
        "observed_on": observed, "captures": metadata, "source_organization_id": ORG,
        "school": school, "coaches": dataset.coaches, "lineage": dataset.lineage,
        "csv": root.join("data/canonical-schools.csv"), "workbook": path,
        "generation_digest": publication.generation_digest, "verified_sheets": verified.sheets,
        "verified_rows": verified.rows, "national_census_qualified": false});
    write_json(&root.join("workbook-qualification.json"), &evidence)?;
    Ok((evidence, xlsx))
}

pub(super) fn read_csv(
    path: &Path,
    school: &CanonicalSchool,
    captured: &str,
) -> Result<[String; 12]> {
    ensure!(
        std::fs::metadata(path)?.len() <= 1024 * 1024,
        "CSV exceeds qualification read budget"
    );
    let mut reader = ::csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    ensure!(header.len() == 28, "canonical school CSV width differs");
    let mut records = reader.records();
    let row = records
        .next()
        .context("canonical school CSV row absent")??;
    ensure!(
        records.next().is_none() && row.len() == header.len(),
        "CSV must contain exactly one aligned school row"
    );
    ensure!(
        row.get(0) == Some(school.id.as_str()) && row.get(1) == Some(NAME),
        "CSV school identity differs"
    );
    let fields = collect_fields(POSTAL_CSV_HEADERS.map(|name| {
        let index = header
            .iter()
            .position(|cell| cell == name)
            .context("postal CSV header absent")?;
        Ok(row.get(index).context("postal CSV cell absent")?.to_owned())
    }))?;
    verify_positions(&fields, school, captured)?;
    Ok(fields)
}

fn read_xlsx(path: &Path, school: &CanonicalSchool, captured: &str) -> Result<[String; 12]> {
    ensure!(
        std::fs::metadata(path)?.len() <= 16 * 1024 * 1024,
        "XLSX exceeds qualification read budget"
    );
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range("Schools")?;
    ensure!(
        range.height() == 2 && range.width() == 24,
        "Schools XLSX must contain header and one 24-column school"
    );
    let mut rows = range.rows();
    let header = rows.next().context("Schools header absent")?;
    header
        .iter()
        .try_for_each(|cell| literal_text(cell).map(|_| ()))?;
    let row = rows.next().context("Schools row absent")?;
    ensure!(
        row.first().map(literal_text).transpose()? == Some(school.id.as_str())
            && row.get(1).map(literal_text).transpose()? == Some(NAME),
        "Schools XLSX identity differs"
    );
    let fields = collect_fields(POSTAL_HEADERS.map(|name| {
        let index = header
            .iter()
            .position(|cell| matches!(cell, Data::String(value) if value == name))
            .context("postal XLSX header absent")?;
        Ok(literal_text(row.get(index).context("postal XLSX cell absent")?)?.to_owned())
    }))?;
    verify_positions(&fields, school, captured)?;
    Ok(fields)
}

fn collect_fields(fields: [Result<String>; 12]) -> Result<[String; 12]> {
    let mut output = std::array::from_fn(|_| String::new());
    output
        .iter_mut()
        .zip(fields)
        .try_for_each(|(slot, field)| -> Result<()> {
            *slot = field?;
            Ok(())
        })?;
    Ok(output)
}

fn literal_text(cell: &Data) -> Result<&str> {
    match cell {
        Data::String(value) => Ok(value),
        Data::Empty => Ok(""),
        other => Err(anyhow::anyhow!(
            "postal/header XLSX cell is not literal text: {other:?}"
        )),
    }
}
