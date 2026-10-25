use super::envelope::{Descriptor, Envelope};
use super::*;
use std::collections::BTreeMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn refused<T>(result: ReportResult<T>, expected: &str) -> TestResult {
    match result.err().ok_or("unexpected success")? {
        ReportError::Store(census_store::StoreError::Refused { detail }) => {
            check!(eq; detail, expected)
        }
        other => return Err(format!("unexpected error: {other:?}").into()),
    }
    Ok(())
}

fn inventory(dir: &Path) -> TestResult<BTreeMap<String, (usize, String)>> {
    std::fs::read_dir(dir)?
        .map(|entry| {
            let entry = entry?;
            let bytes = std::fs::read(entry.path())?;
            Ok((
                entry.file_name().to_string_lossy().into_owned(),
                (bytes.len(), hash(&bytes)),
            ))
        })
        .collect()
}

fn descriptor(dir: &Path, digest: &str) -> TestResult<Descriptor> {
    Ok(serde_json::from_slice(&proof_objects::read(
        dir, digest, 274_944,
    )?)?)
}

#[test]
fn seeded_writer_byte_and_chunk_refusals_preserve_preexisting_state() -> TestResult {
    for version in 1..=4 {
        let directory = tempfile::tempdir()?;
        let envelope = Envelope::for_version(version)?;
        let mut writer = Chunks::new(directory.path(), envelope)?;
        writer.bytes = envelope.max_bytes - 1;
        writer.write_all(b"a")?;
        check!(eq; writer.bytes, envelope.max_bytes);
        let before = (
            writer.buffer.clone(),
            writer.chunks.clone(),
            hash_state(&writer),
        );
        let error = writer.write_all(b"b").err().ok_or("byte excess admitted")?;
        check!(eq; error.to_string(), "participation proof stream refused");
        let captured = writer.failure.take().ok_or("typed failure lost")?;
        refused(
            Err::<(), _>(captured),
            if version == 4 {
                "participation replay exceeds 16 GiB"
            } else {
                "participation replay exceeds 512 MiB"
            },
        )?;
        check!(eq; writer.buffer, before.0);
        check!(eq; writer.chunks, before.1);
        check!(eq; hash_state(&writer), before.2);
        check!(eq; writer.bytes, envelope.max_bytes);
        check!(inventory(directory.path())?.is_empty());
        writer.bytes = usize::MAX;
        refused(writer.accept(b"x"), "participation replay length overflow")?;
        writer.bytes = 1;
        writer.chunks = vec!["a".repeat(64); envelope.max_chunks];
        let prior = writer.chunks.clone();
        refused(writer.store(), "participation replay chunk count exceeded")?;
        check!(eq; writer.chunks, prior);
        check!(eq; writer.buffer, b"a");
        check!(inventory(directory.path())?.is_empty());
        refused(
            Chunks::new(directory.path(), envelope)?.finish(),
            "participation replay descriptor shape or budget differs",
        )?;
    }
    Ok(())
}

fn hash_state(writer: &Chunks<'_>) -> String {
    format!("{:x}", writer.hash.clone().finalize())
}

#[test]
fn real_three_chunk_serialization_roundtrips_and_reuses_identical_objects() -> TestResult {
    let value = "u".repeat(2 * CHUNK_BYTES + 15);
    let canonical = serde_json::to_vec(&value)?;
    check!(eq; canonical.len(), 2 * CHUNK_BYTES + 17);
    for version in 1..=4 {
        let directory = tempfile::tempdir()?;
        let digest = install(&value, directory.path(), version)?;
        check!(eq; super::version(directory.path(), &digest)?, version);
        let descriptor = descriptor(directory.path(), &digest)?;
        check!(eq; descriptor.version, version);
        check!(eq; descriptor.bytes, canonical.len());
        check!(eq; descriptor.sha256, hash(&canonical));
        check!(eq; descriptor.chunks.len(), 3);
        for (index, expected) in [CHUNK_BYTES, CHUNK_BYTES, 17].into_iter().enumerate() {
            let bytes = proof_objects::read(
                directory.path(),
                &descriptor.chunks[index],
                CHUNK_BYTES as u64,
            )?;
            check!(eq; bytes.len(), expected);
            check!(eq; bytes, canonical[index * CHUNK_BYTES..index * CHUNK_BYTES + expected]);
        }
        check!(eq; load::<String>(directory.path(), &digest, version)?, value);
        check!(eq; load_bytes(directory.path(), &digest, version)?, canonical);
        check!(eq; load_bounded_bytes(directory.path(), &digest, version,
            canonical.len() as u64, canonical.len())?, canonical);
        refused(
            load_bounded_bytes(directory.path(), &digest, version, 1, canonical.len()),
            "participation replay payload length differs",
        )?;
        refused(
            load_bounded_bytes(
                directory.path(),
                &digest,
                version,
                canonical.len() as u64,
                canonical.len() - 1,
            ),
            "participation replay descriptor shape or budget differs",
        )?;
        let before = inventory(directory.path())?;
        check!(eq; install(&value, directory.path(), version)?, digest);
        check!(eq; inventory(directory.path())?, before);
    }
    Ok(())
}

#[test]
fn authenticated_missing_corrupt_oversized_chunks_and_cas_conflict_are_typed() -> TestResult {
    let directory = tempfile::tempdir()?;
    let value = "x".repeat(31);
    let digest = install(&value, directory.path(), 4)?;
    let descriptor = descriptor(directory.path(), &digest)?;
    let path = directory.path().join(&descriptor.chunks[0]);
    let original = std::fs::read(&path)?;
    std::fs::remove_file(&path)?;
    let before = inventory(directory.path())?;
    match load_bytes(directory.path(), &digest, 4)
        .err()
        .ok_or("missing admitted")?
    {
        ReportError::Io {
            path: actual,
            source,
        } => {
            check!(eq; actual, path);
            check!(eq; source.kind(), std::io::ErrorKind::NotFound);
        }
        other => return Err(format!("wrong missing error: {other:?}").into()),
    }
    check!(eq; inventory(directory.path())?, before);
    std::fs::write(&path, vec![b'z'; original.len()])?;
    let before = inventory(directory.path())?;
    match load_bytes(directory.path(), &digest, 4)
        .err()
        .ok_or("corruption admitted")?
    {
        ReportError::ArtifactCorrupt {
            path: actual,
            detail,
        } => {
            check!(eq; actual, path);
            check!(eq; detail, "the frozen object does not hash to its name");
        }
        other => return Err(format!("wrong corruption error: {other:?}").into()),
    }
    match install(&value, directory.path(), 4)
        .err()
        .ok_or("CAS conflict replaced")?
    {
        ReportError::Invariant { detail } => check!(eq; detail, format!(
            "frozen object {} already exists with different bytes; refusing to replace {}",
            path.display(), descriptor.chunks[0])),
        other => return Err(format!("wrong CAS error: {other:?}").into()),
    }
    check!(eq; inventory(directory.path())?, before);
    std::fs::write(&path, vec![0; CHUNK_BYTES + 1])?;
    match load_bytes(directory.path(), &digest, 4)
        .err()
        .ok_or("oversized admitted")?
    {
        ReportError::Resource(crate::report::ExportResourceError::RowTooLarge {
            what,
            bytes,
            max,
        }) => {
            check!(eq; what, "frozen object");
            check!(eq; bytes, CHUNK_BYTES as u64 + 1);
            check!(eq; max, CHUNK_BYTES as u64);
        }
        other => return Err(format!("wrong oversize error: {other:?}").into()),
    }
    std::fs::write(&path, &original)?;
    check!(eq; load::<String>(directory.path(), &digest, 4)?, value);
    Ok(())
}

#[test]
fn actual_chunk_order_length_and_payload_digest_guards_are_independent() -> TestResult {
    let directory = tempfile::tempdir()?;
    let value = "x".repeat(2 * CHUNK_BYTES + 15);
    let digest = install(&value, directory.path(), 4)?;
    let mut descriptor = descriptor(directory.path(), &digest)?;
    descriptor.chunks.swap(0, 1);
    let swapped = proof_objects::install(directory.path(), &serde_json::to_vec(&descriptor)?)?;
    let before = inventory(directory.path())?;
    refused(
        load_bytes(directory.path(), &swapped, 4),
        "participation replay payload digest differs",
    )?;
    check!(eq; inventory(directory.path())?, before);
    descriptor.chunks.swap(0, 1);
    descriptor.sha256 = "f".repeat(64);
    let changed = proof_objects::install(directory.path(), &serde_json::to_vec(&descriptor)?)?;
    refused(
        load_bytes(directory.path(), &changed, 4),
        "participation replay payload digest differs",
    )?;
    descriptor.chunks.pop();
    let dropped = proof_objects::install(directory.path(), &serde_json::to_vec(&descriptor)?)?;
    refused(
        load_bytes(directory.path(), &dropped, 4),
        "participation replay descriptor shape or budget differs",
    )?;
    let short = proof_objects::install(directory.path(), b"short")?;
    descriptor.chunks.push(short);
    let truncated = proof_objects::install(directory.path(), &serde_json::to_vec(&descriptor)?)?;
    refused(
        load_bytes(directory.path(), &truncated, 4),
        "participation replay chunk boundary differs",
    )?;
    check!(eq; load::<String>(directory.path(), &digest, 4)?, value);
    Ok(())
}
