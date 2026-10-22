use super::{corrupt_cell, IncludedFixture, TestResult, WorkbookMutation};
use crate::export::{ExportDataset, ExportJobRequest};
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::ProfileSelectionV1;
use census_store::{RetainedCaptureRootSchemaV1, Table};
use std::path::PathBuf;

struct Published {
    fixture: IncludedFixture,
    workbook: PathBuf,
    payload_digest: String,
}

fn publish() -> TestResult<Published> {
    let mut fixture = IncludedFixture::new()?;
    let before = fixture
        .store
        .snapshot()
        .tables_digest(&[Table::Athletes, Table::Performances])?;
    let packet = crate::export::profile_pr_test_source::packet("9501", "2026-09-30T00:50:27Z")?;
    fixture.store.journal_done(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        packet.reference()?.as_str(),
        &packet,
    )?;
    let payload_digest = fixture
        .store
        .snapshot()
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
            packet.reference()?.as_str(),
            &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new(),
        )?
        .ok_or("retained profile PR packet")?
        .root()
        .payload
        .digest
        .as_str()
        .to_owned();
    let selection = ProfileSelectionV1::new(
        fixture.store.store_identity()?,
        fixture
            .dataset
            .selected_profile_id()
            .ok_or("profile")?
            .clone(),
    );
    let request = ExportJobRequest::SelectedPublicationV1 { selection };
    fixture.dataset =
        ExportDataset::for_job_request(&fixture.store, "reported-profile-pr-test", &request)?;
    check!(eq; fixture.store.snapshot().tables_digest(&[Table::Athletes, Table::Performances])?, before);
    fixture.options.out = Some(fixture.directory.path().join("profile-pr-publication"));
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
    let replayed =
        ExportDataset::for_job_request(&fixture.store, "reported-profile-pr-test", &request)?;
    let repeated =
        crate::workbook::build_from(&replayed, &fixture.store, &fixture.options, &censuses)?;
    check!(eq; repeated, workbook);
    Ok(Published {
        fixture,
        workbook,
        payload_digest,
    })
}

#[test]
fn source_pr_publication_retains_every_filter_row_and_survives_store_absence() -> TestResult {
    let Published {
        fixture,
        workbook,
        payload_digest: _,
    } = publish()?;
    let generation = workbook.parent().ok_or("generation")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision").and_then(serde_json::Value::as_u64), Some(13));
    check!(eq; manifest.pointer("/normalized_csv_rows/source_profile_prs.csv").and_then(serde_json::Value::as_u64), Some(46));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(serde_json::Value::as_u64), Some(0));
    check!(eq; manifest.pointer("/normalized_csv_rows/reported_performances.csv").and_then(serde_json::Value::as_u64), Some(3));
    let mut book: Xlsx<_> = open_workbook(&workbook)?;
    let sheet = book.worksheet_range("Athletes")?;
    let text = sheet
        .rows()
        .skip(1)
        .flat_map(|row| row.iter())
        .map(ToString::to_string)
        .find(|text| text.contains("Source-declared PR;"))
        .ok_or("source-declared PR cell")?;
    check!(eq; text.lines().count(), 46);
    for marker in [
        "Mode: all; Level: all",
        "Mode: indoor; Level: hs",
        "Mode: outdoor; Level: ms",
        "Mode: outdoor; Level: open",
        "Mode: outdoor; Level: youth",
        "Locator:",
        "22.23",
        "Feb 15, 2025",
    ] {
        check!(text.contains(marker));
    }
    let commitment = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("frozen archive commitment")?;
    let root = fixture.directory.path().join("selected-store");
    drop(fixture.dataset);
    drop(fixture.store);
    std::fs::rename(&root, fixture.directory.path().join("offline-source-store"))?;
    check!(!root.exists());
    let frozen = ExportDataset::reopen_frozen_with_digest(
        &generation.join("frozen-input.json"),
        commitment,
    )?;
    crate::workbook::verify::verify_frozen(&workbook, &frozen, &fixture.options)?;
    crate::workbook::publication::verify_published(&workbook)?;
    Ok(())
}

#[test]
fn source_pr_cell_and_normalized_claim_mutations_fail_independent_readback() -> TestResult {
    let Published {
        fixture, workbook, ..
    } = publish()?;
    let forged = fixture.directory.path().join("forged-source-pr.xlsx");
    corrupt_cell(
        &workbook,
        &forged,
        WorkbookMutation {
            sheet: "Athletes",
            header: "Source-declared PRs",
            row_key: fixture.first.id.as_str(),
            value: "100m: 1.00 invented",
        },
    )?;
    check!(
        crate::workbook::verify::verify_frozen(&forged, &fixture.dataset, &fixture.options)
            .is_err()
    );
    let generation = workbook.parent().ok_or("generation")?;
    let csv = generation.join("source_profile_prs.csv");
    let original = std::fs::read_to_string(&csv)?;
    let changed = original.replacen("22.23", "99.99", 1);
    check!(ne; changed, original);
    std::fs::write(&csv, changed)?;
    check!(crate::workbook::publication::verify_sidecars::verify(
        generation,
        &fixture.dataset,
        &fixture.options,
        13
    )
    .is_err());
    Ok(())
}

#[test]
fn missing_and_changed_profile_source_objects_refuse_frozen_reopen() -> TestResult {
    let Published {
        fixture: _fixture,
        workbook,
        payload_digest,
    } = publish()?;
    let generation = workbook.parent().ok_or("generation")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    let commitment = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("archive commitment")?;
    let archive = generation.join("frozen-input.json");
    let source = generation
        .join("frozen-input.json.proofs")
        .join(&payload_digest);
    let original = std::fs::read(&source)?;
    std::fs::write(&source, b"controlled source damage")?;
    check!(ExportDataset::reopen_frozen_with_digest(&archive, commitment).is_err());
    std::fs::write(&source, &original)?;
    ExportDataset::reopen_frozen_with_digest(&archive, commitment)?;
    std::fs::remove_file(&source)?;
    check!(ExportDataset::reopen_frozen_with_digest(&archive, commitment).is_err());
    Ok(())
}
