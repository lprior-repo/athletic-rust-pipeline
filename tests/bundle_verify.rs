use anyhow::{Context, Result};
use athletic_rust_pipeline::{
    bundle_verify::{verify_bundle, BundleVerificationReport},
    domain::identity::WorkbookDigest,
    runtime::export::EXPORT_HEADERS,
    store::ArtifactStore,
};
use rust_xlsxwriter::Workbook;
fn verify(
    original: &Path,
    output: &Path,
    digest: &WorkbookDigest,
    headers: &[String],
) -> Result<BundleVerificationReport> {
    let root = output
        .parent()
        .map(|parent| parent.join("artifacts"))
        .context("test output has no parent")?;
    let store = ArtifactStore::open(&root)?;
    verify_bundle(original, output, digest, headers, &store)
}
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn workbook(path: &Path, annotated: bool, status: &str, pr_summary: &str) -> Result<()> {
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
                "native.pr_summary" => pr_summary,
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
        "source": {
            "source_key":"Golden:2",
            "sheet":"Golden",
            "excel_row":2,
            "fields":{"Person First":first, "Person Last":"Runner"}
        },
        "discovery":null,
        "identity_artifacts":[],
        "report_digest":null,
        "report":null,
        "assessment":null,
        "profile_artifacts":[],
        "performance_evidence":[]
    });
    fs::write(path, format!("{row}\n"))?;
    Ok(())
}
fn detail_with_fields(path: &Path, fields: serde_json::Value) -> Result<()> {
    let row = json!({
        "source": {
            "source_key":"Golden:2",
            "sheet":"Golden",
            "excel_row":2,
            "fields":fields
        },
        "discovery":null,
        "identity_artifacts":[],
        "report_digest":null,
        "report":null,
        "assessment":null,
        "profile_artifacts":[],
        "performance_evidence":[]
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
    workbook(&original, false, "", "")?;
    workbook(&output, true, "PENDING", "")?;
    detail(&output.with_extension("jsonl"), "Ada")?;
    let digest = WorkbookDigest::parse(&format!("{:x}", Sha256::digest(fs::read(&original)?)))?;
    let headers = EXPORT_HEADERS
        .iter()
        .map(|header| (*header).to_owned())
        .collect::<Vec<_>>();
    verify(&original, &output, &digest, &headers)?;

    // When an annotation claims acceptance but the retained evidence is pending,
    // then original-cell preservation alone must not allow the bundle to pass.
    workbook(&output, true, "ACCEPTED", "")?;
    anyhow::ensure!(verify(&original, &output, &digest, &headers).is_err());

    // When an unprocessed row claims performance unsupported by its sidecar,
    // then preserving identity and terminal status must not validate that claim.
    workbook(
        &output,
        true,
        "PENDING",
        r#"{"fabricated_personal_best":"0.01 seconds"}"#,
    )?;
    anyhow::ensure!(verify(&original, &output, &digest, &headers).is_err());

    // When an original field is replaced by a declared annotation with the
    // same output value, exact source-column binding must still reject it.
    workbook(&output, true, "PENDING", "")?;
    detail_with_fields(
        &output.with_extension("jsonl"),
        json!({"Person Last":"Runner", "native.source_key":"Golden:2"}),
    )?;
    anyhow::ensure!(verify(&original, &output, &digest, &headers).is_err());

    // When the sidecar is swapped to another identity, then binding also fails.
    workbook(&output, true, "PENDING", "")?;
    detail(&output.with_extension("jsonl"), "Other")?;
    anyhow::ensure!(verify(&original, &output, &digest, &headers).is_err());
    Ok(())
}
