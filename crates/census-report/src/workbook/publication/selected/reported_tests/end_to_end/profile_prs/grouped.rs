use super::*;
use crate::report::ReportError;
use std::path::Path;

#[path = "grouped/expectations.rs"]
mod expectations;
pub(super) use expectations::{cell, csv_records, expectation, texts, Capture};

#[test]
fn three_capture_grouping_keeps_every_claim_and_replays_without_source_store() -> TestResult {
    let published = publish_captures(3)?;
    let Published {
        fixture,
        workbook,
        expected,
        ..
    } = published;
    let (detailed, grouped) = texts(&expected);
    check!(detailed.encode_utf16().count() > 32_767);
    check!(grouped.encode_utf16().count() <= 32_767);
    check!(eq; expected.len(), 3);
    check!(eq; expected.iter().flat_map(|capture| capture.rows.iter()).count(), 138);
    check!(eq; expected.first().ok_or("first capture")?.rows.as_slice(), expected.last().ok_or("last capture")?.rows.as_slice());
    check!(eq; cell(&workbook, fixture.first.id.as_str())?, grouped);
    check!(eq; grouped.lines().count(), 141);
    for capture in &expected {
        check!(eq; grouped.lines().filter(|line| *line == capture.header.as_str()).count(), 1);
        let table = fixture
            .dataset
            .reported_profile_prs_for(&fixture.first)
            .find(|table| table.root.key == capture.root.key)
            .ok_or("captured root")?;
        check!(eq; table.rows.len(), 46);
    }
    check!(eq; fixture.dataset.reported_profile_prs_for(&fixture.first).map(|table| table.root.key.as_str()).collect::<Vec<_>>(), expected.iter().map(|capture| capture.root.key.as_str()).collect::<Vec<_>>());
    let generation = workbook.parent().ok_or("generation")?;
    csv_records(generation, &expected)?;
    let lineage = fixture.dataset.lineage.clone();
    let athlete_id = fixture.first.id.clone();
    let root = fixture.store.root().to_path_buf();
    let IncludedFixture {
        directory,
        store,
        dataset,
        options,
        ..
    } = fixture;
    drop(dataset);
    drop(store);
    check!(eq; root, directory.path().join("selected-store"));
    std::fs::remove_dir_all(&root)?;
    check!(!root.exists());
    let opened = crate::workbook::publication::open_publication(
        &workbook,
        &lineage,
        crate::report::Scope::AllSources,
        census_domain::model::GradYear::CO2027,
    )?;
    let athlete = opened
        .dataset
        .athletes
        .iter()
        .find(|athlete| athlete.id == athlete_id)
        .ok_or("cold athlete")?;
    check!(eq; opened.dataset.reported_profile_prs_for(athlete).count(), 3);
    crate::workbook::verify::verify_frozen(&workbook, &opened.dataset, &options)?;
    crate::workbook::publication::verify_published(&workbook)?;
    csv_records(generation, &expected)?;
    cold_proof_refusals(&workbook, &expected)?;
    Ok(())
}

#[test]
fn grouped_cell_and_late_csv_claim_tampering_fail_direct_semantic_verification() -> TestResult {
    let Published {
        fixture,
        workbook,
        expected,
        ..
    } = publish_captures(3)?;
    let (detailed, original) = texts(&expected);
    check!(detailed.encode_utf16().count() > 32_767);
    check!(original.encode_utf16().count() <= 32_767);
    check!(eq; cell(&workbook, fixture.first.id.as_str())?, original);
    let first = expected.first().ok_or("first capture")?;
    let without_row = original
        .lines()
        .enumerate()
        .filter(|(index, _)| *index != 1)
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    let values = [
        original.clone(),
        original.replacen(&first.root.key, &"0".repeat(64), 1),
        without_row,
        original.replacen("22.23", "99.99", 1),
        original.replacen("Locator: ", "Locator: 999:", 1),
    ];
    let forged = fixture.directory.path().join("changed-grouped.xlsx");
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            check!(ne; value, &original);
        }
        corrupt_cell(
            &workbook,
            &forged,
            WorkbookMutation {
                sheet: "Athletes",
                header: "Source-declared PRs",
                row_key: fixture.first.id.as_str(),
                value,
            },
        )?;
        let verified =
            crate::workbook::verify::verify_frozen(&forged, &fixture.dataset, &fixture.options);
        if index == 0 {
            verified?;
        } else {
            check!(
                matches!(verified, Err(ReportError::SemanticMismatch { path, .. }) if path == forged)
            );
        }
    }
    late_csv(&fixture, &workbook, &expected)
}

fn late_csv(fixture: &IncludedFixture, workbook: &Path, expected: &[Capture]) -> TestResult {
    let generation = workbook.parent().ok_or("generation")?;
    let path = generation.join("source_profile_prs.csv");
    let original = std::fs::read(&path)?;
    let mut reader = csv::Reader::from_reader(original.as_slice());
    let header = reader.headers()?.clone();
    let mut rows = reader
        .records()
        .map(|row| Ok(row?.iter().map(str::to_owned).collect::<Vec<_>>()))
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; rows.len(), 138);
    let last = rows.last_mut().ok_or("late claim")?;
    check!(eq; last.get(11).ok_or("root")?, &serde_json::to_string(&expected.last().ok_or("last root")?.root)?);
    let mark = last.get_mut(4).ok_or("mark field")?;
    check!(ne; mark.as_str(), "999.99");
    *mark = "999.99".to_owned();
    let mut writer = csv::Writer::from_path(&path)?;
    writer.write_record(&header)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    drop(writer);
    let refused = crate::workbook::publication::verify_sidecars::verify(
        generation,
        &fixture.dataset,
        &fixture.options,
        13,
    );
    std::fs::write(&path, &original)?;
    check!(
        matches!(refused, Err(ReportError::Invariant { detail }) if detail.contains("source_profile_prs.csv record 139 column 4:"))
    );
    crate::workbook::publication::verify_sidecars::verify(
        generation,
        &fixture.dataset,
        &fixture.options,
        13,
    )?;
    crate::workbook::publication::verify_published(workbook)?;
    csv_records(generation, expected)
}

fn cold_proof_refusals(workbook: &Path, expected: &[Capture]) -> TestResult {
    let generation = workbook.parent().ok_or("generation")?;
    let archive = generation.join("frozen-input.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    let digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("frozen commitment")?;
    let source = generation.join("frozen-input.json.proofs").join(
        expected
            .last()
            .ok_or("second capture")?
            .root
            .payload
            .digest
            .as_str(),
    );
    let original = std::fs::read(&source)?;
    let mut changed = original.clone();
    *changed.first_mut().ok_or("nonempty source")? ^= 1;
    std::fs::write(&source, &changed)?;
    let refused = ExportDataset::reopen_frozen_with_digest(&archive, digest);
    std::fs::write(&source, &original)?;
    check!(
        matches!(refused, Err(ReportError::ArtifactCorrupt { path, detail }) if path == source && detail == "the frozen object does not hash to its name")
    );
    crate::workbook::publication::verify_published(workbook)?;
    std::fs::remove_file(&source)?;
    let refused = ExportDataset::reopen_frozen_with_digest(&archive, digest);
    std::fs::write(&source, &original)?;
    check!(
        matches!(refused, Err(ReportError::Invariant { detail }) if detail == "selected proof directory differs from its authenticated object inventory")
    );
    crate::workbook::publication::verify_published(workbook)?;
    Ok(())
}

#[test]
fn complete_grouped_overflow_refuses_publication_and_preserves_previous_generation() -> TestResult {
    let Published {
        mut fixture,
        workbook,
        ..
    } = publish()?;
    let before = protected(&fixture, &workbook)?;
    let expected = retain(&fixture, 8)?;
    recapture(&mut fixture, "reported-profile-pr-grouped-too-large")?;
    check!(eq; fixture.dataset.reported_profile_prs_for(&fixture.first).count(), 8);
    let (detailed, grouped) = texts(&expected);
    check!(detailed.encode_utf16().count() > 32_767);
    check!(grouped.encode_utf16().count() > 32_767);
    let result = build(&fixture);
    check!(
        matches!(result, Err(ReportError::Invariant { detail }) if detail == "complete grouped source-declared PR claims exceed Excel cell budget; no claims were truncated")
    );
    check!(eq; protected(&fixture, &workbook)?, before);
    check!(eq; crate::workbook::publication::current_workbook(fixture.options.out.as_ref().ok_or("publication root")?)?, workbook);
    crate::workbook::publication::verify_published(&workbook)?;
    Ok(())
}

fn protected(
    fixture: &IncludedFixture,
    workbook: &Path,
) -> TestResult<(PathBuf, Vec<u8>, Vec<u8>)> {
    Ok((
        std::fs::read_link(
            fixture
                .options
                .out
                .as_ref()
                .ok_or("publication root")?
                .join("current"),
        )?,
        std::fs::read(workbook.parent().ok_or("generation")?.join("manifest.json"))?,
        std::fs::read(workbook)?,
    ))
}
