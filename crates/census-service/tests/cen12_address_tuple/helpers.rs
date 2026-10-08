use super::{expected, TestResult};
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::school_directory::SchoolDirectoryEntry;
use census_domain::UsJurisdiction;
use census_service::school_address::SchoolAddressArgs;
use census_store::{Store, Table};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn args(out: &Path) -> SchoolAddressArgs {
    SchoolAddressArgs {
        ccd: Some(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../census-crawl/tests/fixtures/cen12/ccd_tuples.csv"
            )
            .into(),
        ),
        pss: None,
        state_ed_index: Vec::new(),
        state_ed_profile: Vec::new(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        association_directory: Vec::new(),
        out: out.into(),
        baseline: None,
        ledger: None,
        now: None,
        geocode: false,
        validate_postal: false,
    }
}

pub(super) fn seed(store: &Store, entries: &[SchoolDirectoryEntry]) -> TestResult {
    for entry in entries {
        let name = entry.name().ok_or("missing source school name")?.as_str();
        let (school, _) =
            CanonicalSchool::new(UsJurisdiction::NewJersey, name, normalize_name(name), None);
        store.append(Table::Schools, &school)?;
    }
    Ok(())
}

fn cell(row: &[calamine::Data], headers: &[calamine::Data], name: &str) -> TestResult<String> {
    let position = headers
        .iter()
        .position(|value| value.to_string() == name)
        .ok_or_else(|| format!("missing output column {name}"))?;
    Ok(row
        .get(position)
        .ok_or("output position absent")?
        .to_string())
}

pub(super) fn verify_workbook(path: &Path, entries: &[SchoolDirectoryEntry]) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    let sheet = book.worksheet_range("Schools")?;
    let mut rows = sheet.rows();
    let headers = rows.next().ok_or("school header absent")?;
    let published: BTreeMap<String, Vec<calamine::Data>> = rows
        .map(|row| cell(row, headers, "School").map(|name| (name, row.to_vec())))
        .collect::<TestResult<_>>()?;
    for (entry, expected) in entries.iter().zip(expected()) {
        let name = entry.name().ok_or("source name absent")?.as_str();
        let row = published
            .get(name)
            .ok_or("source school lost in workbook")?;
        for (header, expected) in [
            "Postal Street",
            "Postal Second Line",
            "Postal City",
            "Postal State",
            "Postal ZIP",
            "Postal Address Kind",
        ]
        .into_iter()
        .zip(expected)
        {
            check!(eq; cell(row, headers, header)?, expected);
        }
    }
    Ok(())
}

pub(super) fn verify_retained_notes(generation: &Path) -> TestResult {
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(
        generation.join("current/pipeline_report.json"),
    )?)?;
    let lane = report
        .get("lanes")
        .and_then(|value| value.as_array())
        .and_then(|lanes| lanes.first())
        .ok_or("published source lane missing")?;
    let notes = lane
        .get("note_rows")
        .and_then(|value| value.as_array())
        .ok_or("published notes missing")?;
    let mailing = notes
        .iter()
        .find(|note| {
            note.get("line").and_then(|value| value.as_u64()) == Some(2)
                && note.get("field").and_then(|value| value.as_str())
                    == Some("unselected mailing address")
        })
        .ok_or("published unselected mailing claim lost")?;
    check!(eq; mailing.get("detail").and_then(|value| value.as_str()),
        Some("retained in source capture: street=\"PO Box 900\"; line2=\"Mail Suite\"; city=\"New York\"; state=\"NY\"; zip=\"10001\"; plus4=\"1234\""));
    let invalid = notes
        .iter()
        .find(|note| {
            note.get("line").and_then(|value| value.as_u64()) == Some(6)
                && note.get("field").and_then(|value| value.as_str()) == Some("address zip")
        })
        .ok_or("published denied ZIP claim lost")?;
    check!(invalid
        .get("detail")
        .and_then(|value| value.as_str())
        .is_some_and(|detail| detail.contains("BAD")));
    Ok(())
}

pub(super) fn verify_corpus_csv(generation: &Path) -> TestResult {
    let mut csv = csv::Reader::from_path(generation.join("current/school_directory.csv"))?;
    let headers = csv.headers()?.clone();
    let kind = headers
        .iter()
        .position(|header| header == "address_kind")
        .ok_or("corpus CSV lost kind")?;
    let kinds = csv
        .records()
        .map(|record| {
            Ok(record?
                .get(kind)
                .ok_or("corpus address kind position absent")?
                .to_owned())
        })
        .collect::<TestResult<Vec<String>>>()?;
    check!(eq; kinds, expected().map(|fields| fields.get(5).copied().map_or("", core::convert::identity).to_owned()));
    Ok(())
}
