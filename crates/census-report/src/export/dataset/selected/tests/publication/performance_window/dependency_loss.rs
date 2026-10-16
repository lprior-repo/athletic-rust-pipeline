use super::{readback::Expected, ExportDataset, TestResult};
use crate::export::qualification::participation::ParticipationReplay;
use crate::report::ReportError;
use serde_json::Value;
use std::path::Path;

pub(super) fn rejects(generation: &Path, digest: &str, expected: &Expected) -> TestResult {
    let copied = super::super::super::temporary()?;
    let destination = copied.path().join("copied-published-generation");
    copy_generation(generation, &destination)?;
    let frozen = destination.join("frozen-input.json");
    let original_root = std::fs::read(&frozen)?;
    let archive: Value = serde_json::from_slice(&original_root)?;
    let descriptor = archive
        .pointer("/selected_proofs/participation_digest")
        .and_then(Value::as_str)
        .ok_or("anchored participation replay")?;
    let proofs = destination.join("frozen-input.json.proofs");
    let descriptor_bytes = std::fs::read(proofs.join(descriptor))?;
    let metadata: Value = serde_json::from_slice(&descriptor_bytes)?;
    check!(eq; metadata.get("version").and_then(Value::as_u64), Some(2));
    let chunks = metadata
        .get("chunks")
        .and_then(Value::as_array)
        .ok_or("replay chunks")?;
    check!(eq; chunks.len(), 1);
    let chunk = proofs.join(
        chunks
            .first()
            .and_then(Value::as_str)
            .ok_or("fixture replay payload chunk")?,
    );
    let original = std::fs::read(&chunk)?;
    let packet = expected
        .packets
        .values()
        .next()
        .ok_or("authored independent packet")?;
    let encoded_body = packet
        .pointer("/status/body_base64")
        .and_then(Value::as_str)
        .ok_or("status body bytes")?;
    let needle = format!("\"body_base64\":\"{encoded_body}\"");
    let matches = original
        .windows(needle.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == needle.as_bytes()).then_some(offset))
        .collect::<Vec<_>>();
    check!(eq; matches.len(), 1);
    let offset = matches
        .first()
        .copied()
        .and_then(|start| start.checked_add("\"body_base64\":\"".len()))
        .ok_or("exact status body offset")?;
    let mut changed = original.clone();
    let byte = changed.get_mut(offset).ok_or("retained status body byte")?;
    *byte = if *byte == b'A' { b'B' } else { b'A' };
    check!(eq; original.iter().zip(&changed).filter(|(left, right)| left != right).count(), 1);
    std::fs::write(&chunk, &changed)?;
    refused(ParticipationReplay::load(&proofs, descriptor), &chunk)?;
    refused(
        ExportDataset::reopen_frozen_with_digest(&frozen, digest),
        &chunk,
    )?;
    check!(eq; std::fs::read(&chunk)?, changed);
    check!(eq; std::fs::read(&frozen)?, original_root);
    check!(eq; std::fs::read(proofs.join(descriptor))?, descriptor_bytes);
    check!(eq; std::fs::read(destination.join("manifest.json"))?, std::fs::read(generation.join("manifest.json"))?);
    Ok(())
}

fn refused<T>(outcome: Result<T, ReportError>, target: &Path) -> TestResult {
    match outcome {
        Err(ReportError::ArtifactCorrupt { path, .. }) => {
            check!(eq; path, target);
            Ok(())
        }
        Err(other) => {
            Err(format!("independent dependency returned wrong typed refusal: {other:?}").into())
        }
        Ok(_) => Err("corrupted independent status dependency was admitted".into()),
    }
}

fn copy_generation(source: &Path, destination: &Path) -> TestResult {
    std::fs::create_dir(destination)?;
    let mut pending = vec![(source.to_path_buf(), destination.to_path_buf(), 0usize)];
    let mut files = 0usize;
    let mut bytes = 0u64;
    while let Some((source, destination, depth)) = pending.pop() {
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let metadata = entry.file_type()?;
            let target = destination.join(entry.file_name());
            check!(!metadata.is_symlink());
            if metadata.is_dir() {
                check!(depth < 3);
                check!(pending.len() < 16);
                std::fs::create_dir(&target)?;
                pending.push((
                    entry.path(),
                    target,
                    depth.checked_add(1).ok_or("copy depth")?,
                ));
            } else {
                check!(metadata.is_file());
                files = files.checked_add(1).ok_or("copy file count")?;
                bytes = bytes
                    .checked_add(entry.metadata()?.len())
                    .ok_or("copy bytes")?;
                check!(files <= 1024);
                check!(bytes <= 32 * 1024 * 1024);
                std::fs::copy(entry.path(), target)?;
            }
        }
    }
    Ok(())
}
