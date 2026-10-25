use super::*;
use crate::export::qualification::participation::archive::ParticipationReplay;
use crate::report::{ExportResourceError, ReportError};

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

fn fixture(version: u32, chunks: usize) -> Descriptor {
    Descriptor {
        version,
        bytes: chunks * CHUNK_BYTES,
        sha256: "b".repeat(64),
        chunks: vec!["a".repeat(64); chunks],
    }
}

fn install(dir: &Path, value: &Descriptor) -> TestResult<String> {
    Ok(proof_objects::install(dir, &serde_json::to_vec(value)?)?)
}

fn oversized<T>(result: ReportResult<T>, bytes: u64, max: u64) -> TestResult {
    match result.err().ok_or("oversized admitted")? {
        ReportError::Resource(ExportResourceError::RowTooLarge {
            what,
            bytes: actual,
            max: limit,
        }) => {
            check!(eq; what, "frozen object");
            check!(eq; actual, bytes);
            check!(eq; limit, max);
        }
        other => return Err(format!("unexpected error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn exact_policy_and_checked_production_relations_cover_both_envelopes() -> TestResult {
    for version in 1..=4 {
        let e = Envelope::for_version(version)?;
        let expected = if version == 4 {
            (17_179_869_184, 4096, 274_944)
        } else {
            (536_870_912, 128, 32_768)
        };
        check!(eq; (e.max_bytes, e.max_chunks, e.descriptor_bytes), expected);
        check!(eq; e.version, version);
        check!(eq; e.admit_bytes(e.max_bytes - 1, 1)?, e.max_bytes);
        for (bytes, count) in [
            (1, 1),
            (CHUNK_BYTES - 1, 1),
            (CHUNK_BYTES, 1),
            (CHUNK_BYTES + 1, 2),
            (e.max_bytes, e.max_chunks),
            (e.max_bytes + 1, e.max_chunks + 1),
        ] {
            check!(eq; chunk_count(bytes)?, count);
        }
    }
    check!(eq; CHUNK_BYTES, 4_194_304);
    check!(eq; descriptor_limit(4096)?, 274_944);
    refused(
        descriptor_limit(usize::MAX),
        "participation replay descriptor limit overflow",
    )?;
    for version in [0, 5, u32::MAX] {
        refused(
            Envelope::for_version(version),
            "unsupported participation replay descriptor version",
        )?;
    }
    Ok(())
}

#[test]
fn both_large_descriptor_gates_admit_metadata_before_expected_length_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    for count in [1480, 4096] {
        let expected = fixture(4, count);
        let raw = serde_json::to_vec(&expected)?;
        check!(raw.len() > 32_768 && raw.len() <= 274_944);
        let digest = install(directory.path(), &expected)?;
        check!(eq; version(directory.path(), &digest)?, 4);
        check!(eq; read(directory.path(), &digest, 4, 17_179_869_184, None)?, expected);
        // Expected-length mismatch precedes try_reserve_exact and chunk reads.
        // No absent-chunk maximum loader invocation or 16-GiB reservation.
        refused(
            super::super::load_bounded_bytes(directory.path(), &digest, 4, 1, 17_179_869_184),
            "participation replay payload length differs",
        )?;
    }
    Ok(())
}

#[test]
fn descriptor_shape_count_canonical_and_raw_byte_bounds_refuse_before_payload() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut cases = vec![fixture(4, 4097), fixture(4, 1)];
    cases[1].bytes = 0;
    let mut over = fixture(4, 4096);
    over.bytes += 1;
    cases.push(over);
    let mut mismatch = fixture(4, 1);
    mismatch.chunks.clear();
    cases.push(mismatch);
    for case in cases {
        let digest = install(directory.path(), &case)?;
        refused(
            read(directory.path(), &digest, 4, 17_179_869_184, None),
            "participation replay descriptor shape or budget differs",
        )?;
    }
    for length in [274_944, 274_945] {
        let mut raw = serde_json::to_vec(&fixture(4, 1))?;
        raw.resize(length, b' ');
        let digest = proof_objects::install(directory.path(), &raw)?;
        if length == 274_944 {
            refused(
                read(directory.path(), &digest, 4, 17_179_869_184, None),
                "participation replay descriptor shape or budget differs",
            )?;
        } else {
            oversized(
                read(directory.path(), &digest, 4, 17_179_869_184, None),
                length as u64,
                274_944,
            )?;
            oversized(version(directory.path(), &digest), length as u64, 274_944)?;
        }
    }
    Ok(())
}

#[test]
fn reader_keeps_expected_length_before_underlying_chunk_name_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut value = fixture(3, 1);
    value.bytes = 1;
    value.chunks[0] = "A".repeat(64);
    let digest = install(directory.path(), &value)?;
    refused(
        super::super::load_bounded_bytes(directory.path(), &digest, 3, 2, 536_870_912),
        "participation replay payload length differs",
    )?;
    match super::super::load_bounded_bytes(directory.path(), &digest, 3, 1, 536_870_912)
        .err()
        .ok_or("invalid name admitted")?
    {
        ReportError::Invariant { detail } => check!(eq; detail, format!(
            "frozen object name {} is not a lowercase sha256 digest", value.chunks[0])),
        other => return Err(format!("wrong name refusal: {other:?}").into()),
    }
    let writer = Envelope::for_version(3)?;
    refused(
        writer.encode(&value),
        "participation replay descriptor shape or budget differs",
    )?;
    value.chunks[0] = "a".repeat(64);
    value.sha256 = "bad".into();
    refused(
        writer.encode(&value),
        "participation replay descriptor shape or budget differs",
    )?;
    Ok(())
}

#[test]
fn legacy_attachment_and_unknown_dispatch_keep_original_admission() -> TestResult {
    let directory = tempfile::tempdir()?;
    for legacy in 1..=3 {
        let mut raw = serde_json::to_vec(&fixture(legacy, 1))?;
        raw.resize(32_769, b' ');
        let digest = proof_objects::install(directory.path(), &raw)?;
        oversized(version(directory.path(), &digest), 32_769, 32_768)?;
        oversized(
            read(directory.path(), &digest, legacy, 536_870_912, None),
            32_769,
            32_768,
        )?;
        let digest = install(directory.path(), &fixture(legacy, 129))?;
        refused(
            read(directory.path(), &digest, legacy, 536_870_912, None),
            "participation replay descriptor shape or budget differs",
        )?;
        refused(
            read(directory.path(), &digest, legacy, 536_870_913, None),
            "participation replay exceeds absolute maximum",
        )?;
    }
    for unknown in [0, 5, u32::MAX] {
        let digest = install(directory.path(), &fixture(unknown, 1))?;
        check!(eq; version(directory.path(), &digest)?, unknown);
        refused(
            ParticipationReplay::load(directory.path(), &digest),
            "unsupported participation replay descriptor version",
        )?;
    }
    for malformed in [
        b"{".as_slice(),
        b"{}",
        b"{\"version\":4294967296}",
        b"{\"version\":4,\"bytes\":1,\"sha256\":\"x\",\"chunks\":[],\"extra\":1}",
    ] {
        let digest = proof_objects::install(directory.path(), malformed)?;
        match version(directory.path(), &digest)
            .err()
            .ok_or("malformed admitted")?
        {
            ReportError::Store(census_store::StoreError::Json { detail, .. }) => {
                check!(eq; detail, "participation replay object");
            }
            other => return Err(format!("unexpected parse error: {other:?}").into()),
        }
    }
    Ok(())
}
