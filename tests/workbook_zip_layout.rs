use anyhow::{bail, Context, Result};
use athletic_rust_pipeline::{model::SOURCE_HEADERS, workbook_ingest};
use rust_xlsxwriter::Workbook;
use std::{fs, io::Write, path::Path};
use tempfile::tempdir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

fn write_source_workbook(path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("Synthetic")?;
    SOURCE_HEADERS
        .iter()
        .enumerate()
        .try_for_each(|(column, header)| {
            let column = u16::try_from(column)?;
            worksheet
                .write_string(0, column, *header)
                .map(|_| ())
                .map_err(anyhow::Error::from)
        })?;
    [
        "Runner",
        "Synthetic",
        "synthetic@example.invalid",
        "100 Fictional Way",
        "Fictional City",
        "ZZ",
        "00000",
        "2026-01-01",
        "Track",
        "0",
        "2026-01-01",
        "synthetic-fixture",
        "Fictional High",
    ]
    .iter()
    .enumerate()
    .try_for_each(|(column, value)| {
        let column = u16::try_from(column)?;
        worksheet
            .write_string(1, column, *value)
            .map(|_| ())
            .map_err(anyhow::Error::from)
    })?;
    workbook.save(path).map_err(anyhow::Error::from)
}

#[test]
fn rust_xlsxwriter_descriptor_workbook_is_accepted() -> Result<()> {
    let directory = tempdir()?;
    let path = directory.path().join("descriptor.xlsx");
    write_source_workbook(&path)?;
    let mut records = Vec::new();
    let stats = workbook_ingest::visit_records(&path, |record| {
        records.push(record);
        Ok(())
    })?;
    anyhow::ensure!(
        stats.actual_data_rows == 1,
        "left={:?} right={:?}",
        &stats.actual_data_rows,
        &1
    );
    {
        let left_value = &(records.first().map(|record| record.source_key.as_str()));
        let right_value = &(Some("Synthetic:2"));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    Ok(())
}

#[test]
fn zip64_streaming_descriptors_preserve_workbook_fields() -> Result<()> {
    let directory = tempdir()?;
    let original = directory.path().join("source.xlsx");
    let destination = directory.path().join("zip64.xlsx");
    write_source_workbook(&original)?;
    let mut original = zip::ZipArchive::new(fs::File::open(&original)?)?;
    let mut output = ZipWriter::new_stream(fs::File::create(&destination)?);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true);
    (0..original.len()).try_for_each(|index| -> Result<()> {
        let mut entry = original.by_index(index)?;
        output.start_file(entry.name(), options)?;
        std::io::copy(&mut entry, &mut output)?;
        Ok(())
    })?;
    output.finish()?;
    let mut records = Vec::new();
    workbook_ingest::visit_records(&destination, |record| {
        records.push(record);
        Ok(())
    })?;
    {
        let left_value = &(records
            .first()
            .and_then(|record| record.fields.get("Person Email"))
            .map(String::as_str));
        let right_value = &(Some("synthetic@example.invalid"));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    Ok(())
}

#[test]
fn duplicate_central_names_are_rejected_by_visit_records() -> Result<()> {
    let directory = tempdir()?;
    let path = directory.path().join("duplicate.xlsx");
    write_source_workbook(&path)?;
    let file = fs::OpenOptions::new().read(true).write(true).open(&path)?;
    let mut archive = ZipWriter::new_append(file)?;
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    archive.start_file("fixture-a.txt", options)?;
    archive.write_all(b"synthetic evidence")?;
    archive.start_file("fixture-b.txt", options)?;
    archive.write_all(b"synthetic evidence")?;
    archive.finish()?;
    {
        let left_value = &(workbook_ingest::visit_records(&path, |_| Ok(()))?.actual_data_rows);
        let right_value = &1;
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    let mut bytes = fs::read(&path)?;
    let positions = bytes
        .windows(b"fixture-b.txt".len())
        .enumerate()
        .filter_map(|(offset, value)| (value == b"fixture-b.txt").then_some(offset))
        .collect::<Vec<_>>();
    anyhow::ensure!(
        positions.len() == 2,
        "synthetic local and central names must both be present — left={:?} right={:?}",
        &positions.len(),
        &2
    );
    positions.into_iter().try_for_each(|offset| -> Result<()> {
        bytes
            .get_mut(offset..offset + b"fixture-a.txt".len())
            .context("synthetic ZIP name bounds")?
            .copy_from_slice(b"fixture-a.txt");
        Ok(())
    })?;
    fs::write(&path, bytes)?;
    anyhow::ensure!(workbook_ingest::visit_records(&path, |_| Ok(())).is_err());
    Ok(())
}

#[test]
fn local_header_name_mismatch_is_rejected_by_visit_records() -> Result<()> {
    let directory = tempdir()?;
    let path = directory.path().join("mismatch.xlsx");
    write_source_workbook(&path)?;
    let mut bytes = fs::read(&path)?;
    let signature = [0x50, 0x4b, 0x03, 0x04];
    let local = bytes
        .windows(signature.len())
        .position(|window| window == signature)
        .ok_or_else(|| anyhow::anyhow!("synthetic workbook has no local header"))?;
    let name = local
        .checked_add(30)
        .ok_or_else(|| anyhow::anyhow!("local name offset overflow"))?;
    if bytes.get_mut(name).is_none() {
        bail!("synthetic workbook local name is truncated");
    }
    bytes[name] ^= 1;
    fs::write(&path, bytes)?;
    anyhow::ensure!(workbook_ingest::visit_records(&path, |_| Ok(())).is_err());
    Ok(())
}
