use super::{corrupt_cell, IncludedFixture, TestResult, WorkbookMutation};
use crate::export::{ExportDataset, ExportJobRequest};
use census_domain::model::ProfileSelectionV1;
use census_store::{RetainedCaptureRootSchemaV1, Table};
use std::path::PathBuf;

#[path = "profile_prs/grouped.rs"]
mod grouped;

struct Published {
    fixture: IncludedFixture,
    workbook: PathBuf,
    payload_digest: String,
    expected: Vec<grouped::Capture>,
}

fn publish() -> TestResult<Published> {
    publish_captures(1)
}

fn publish_captures(count: usize) -> TestResult<Published> {
    let mut fixture = IncludedFixture::new()?;
    let before = fixture
        .store
        .snapshot()
        .tables_digest(&[Table::Athletes, Table::Performances])?;
    let expected = retain(&fixture, count)?;
    let payload_digest = expected
        .first()
        .ok_or("retained capture")?
        .root
        .payload
        .digest
        .as_str()
        .to_owned();
    recapture(&mut fixture, "reported-profile-pr-test")?;
    check!(crate::report::Derivation::for_publication(
        &fixture.dataset,
        fixture.options.scope,
        fixture.options.grad_year,
        &fixture.options.jurisdictions,
    )?
    .athletes()
    .is_empty());
    check!(eq; fixture.store.snapshot().tables_digest(&[Table::Athletes, Table::Performances])?, before);
    fixture.options.out = Some(fixture.directory.path().join("profile-pr-publication"));
    let workbook = build(&fixture)?;
    recapture(&mut fixture, "reported-profile-pr-test")?;
    check!(eq; build(&fixture)?, workbook);
    Ok(Published {
        fixture,
        workbook,
        payload_digest,
        expected,
    })
}

fn retain(fixture: &IncludedFixture, count: usize) -> TestResult<Vec<grouped::Capture>> {
    check!(count > 0 && count <= 8);
    let base = chrono::DateTime::parse_from_rfc3339("2026-09-30T00:50:27Z")?;
    let mut expected = Vec::new();
    for index in 0..count {
        let acquired = base
            .checked_add_signed(chrono::TimeDelta::seconds(i64::try_from(index)?))
            .ok_or("capture time")?;
        let packet = crate::export::profile_pr_test_source::packet("9501", &acquired.to_rfc3339())?;
        let key = packet.reference()?;
        fixture.store.journal_done(
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            key.as_str(),
            &packet,
        )?;
        let held = fixture
            .store
            .snapshot()
            .read_retained_capture_root_v1(
                RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
                key.as_str(),
                &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new(),
            )?
            .ok_or("retained profile PR packet")?;
        expected.push(grouped::expectation(
            &packet,
            held.root().clone(),
            fixture.first.id.as_str(),
        )?);
    }
    expected.sort_by(|left, right| left.root.key.cmp(&right.root.key));
    check!(eq; expected.iter().map(|capture| capture.root.key.as_str()).collect::<std::collections::BTreeSet<_>>().len(), count);
    Ok(expected)
}

fn recapture(fixture: &mut IncludedFixture, key: &str) -> TestResult {
    let selection = ProfileSelectionV1::new(
        fixture.store.store_identity()?,
        fixture
            .dataset
            .selected_profile_id()
            .ok_or("profile")?
            .clone(),
    );
    fixture.dataset = ExportDataset::for_job_request(
        &fixture.store,
        key,
        &ExportJobRequest::SelectedPublicationV1 { selection },
    )?;
    Ok(())
}

fn build(fixture: &IncludedFixture) -> crate::report::ReportResult<PathBuf> {
    let censuses = crate::workbook::Censuses::of(
        &fixture.dataset,
        &fixture.store.out_dir(),
        &fixture.options.jurisdictions,
    )?;
    crate::workbook::build_from(
        &fixture.dataset,
        &fixture.store,
        &fixture.options,
        &censuses,
    )
}

#[test]
fn source_pr_publication_retains_every_filter_row_and_survives_store_absence() -> TestResult {
    let Published {
        fixture,
        workbook,
        expected,
        ..
    } = publish()?;
    let generation = workbook.parent().ok_or("generation")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision").and_then(serde_json::Value::as_u64), Some(13));
    check!(eq; manifest.pointer("/normalized_csv_rows/source_profile_prs.csv").and_then(serde_json::Value::as_u64), Some(46));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(serde_json::Value::as_u64), Some(0));
    check!(eq; manifest.pointer("/normalized_csv_rows/reported_performances.csv").and_then(serde_json::Value::as_u64), Some(3));
    let text = grouped::cell(&workbook, fixture.first.id.as_str())?;
    let (detailed, _) = grouped::texts(&expected);
    check!(detailed.encode_utf16().count() <= 32_767);
    check!(eq; text, detailed);
    check!(eq; text.lines().count(), 46);
    grouped::csv_records(generation, &expected)?;
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
    let refused = crate::workbook::publication::verify_sidecars::verify(
        generation,
        &fixture.dataset,
        &fixture.options,
        13,
    );
    std::fs::write(&csv, &original)?;
    check!(refused.is_err());
    crate::workbook::publication::verify_sidecars::verify(
        generation,
        &fixture.dataset,
        &fixture.options,
        13,
    )?;
    Ok(())
}

#[test]
fn missing_and_changed_profile_source_objects_refuse_frozen_reopen() -> TestResult {
    let Published {
        fixture: _fixture,
        workbook,
        payload_digest,
        ..
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
    std::fs::write(&source, &original)?;
    crate::workbook::publication::verify_published(&workbook)?;
    Ok(())
}
