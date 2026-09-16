use anyhow::Result;
use athletic_rust_pipeline::{
    bundle_verify::verify_bundle, domain::identity::WorkbookDigest, runtime::export::EXPORT_HEADERS,
};
use rust_xlsxwriter::Workbook;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn workbook(path: &Path, annotated: bool, status: &str) -> Result<()> {
    let mut book = Workbook::new();
    let sheet = book.add_worksheet().set_name("Golden")?;
    for (column, name) in ["Person First", "Person Last"].into_iter().enumerate() {
        sheet.write_string(0, u16::try_from(column)?, name)?;
    }
    sheet.write_string(1, 0, "Ada")?;
    sheet.write_string(1, 1, "Runner")?;
    if annotated {
        for (offset, header) in EXPORT_HEADERS.iter().enumerate() {
            let column = u16::try_from(offset + 2)?;
            sheet.write_string(0, column, *header)?;
            let value = match *header {
                "native.source_key" => "Golden:2",
                "native.terminal_status" => status,
                "native.eligibility_basis" => "source_workbook_membership",
                _ => "",
            };
            sheet.write_string(1, column, value)?;
        }
    }
    book.save(path)?;
    Ok(())
}

fn detail(path: &Path, first: &str) -> Result<()> {
    let row = json!({
        "source": {"source_key":"Golden:2", "sheet":"Golden", "excel_row":2,
            "fields":{"Person First":first,"Person Last":"Runner"}},
        "report_digest":null,"report":null,"assessment":null,
        "profile_artifacts":[],"performance_evidence":[]
    });
    fs::write(path, format!("{row}\n"))?;
    Ok(())
}

#[test]
fn rejects_annotation_and_sidecar_tampering_despite_preserved_original_cells() -> Result<()> {
    // Given a preserved source row and a consistent pending export bundle.
    let directory = tempfile::tempdir()?;
    let original = directory.path().join("original.xlsx");
    let output = directory.path().join("output.xlsx");
    workbook(&original, false, "")?;
    workbook(&output, true, "PENDING")?;
    detail(&output.with_extension("jsonl"), "Ada")?;
    let digest = WorkbookDigest::parse(&format!("{:x}", Sha256::digest(fs::read(&original)?)))?;
    let headers = EXPORT_HEADERS
        .iter()
        .map(|header| (*header).to_owned())
        .collect::<Vec<_>>();
    let valid = verify_bundle(&original, &output, &digest, &headers)?;
    assert_eq!(valid.fields.matched_row_count, 1);
    assert_eq!(valid.results.pending_rows, 1);

    // When an annotation claims acceptance but the retained evidence is pending,
    // then original-cell preservation alone must not allow the bundle to pass.
    workbook(&output, true, "ACCEPTED")?;
    assert!(verify_bundle(&original, &output, &digest, &headers).is_err());

    // When the sidecar is swapped to another identity, then binding also fails.
    workbook(&output, true, "PENDING")?;
    detail(&output.with_extension("jsonl"), "Other")?;
    assert!(verify_bundle(&original, &output, &digest, &headers).is_err());
    Ok(())
}
