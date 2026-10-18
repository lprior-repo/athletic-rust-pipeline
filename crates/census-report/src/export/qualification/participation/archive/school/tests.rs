use super::{load_school, SchoolClosureDescriptor};
use crate::export::proof_objects;
use crate::report::ReportError;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::public_identity_history::ReplayLimits;
use census_store::StoreError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const LIMIT: usize = ReplayLimits::SMALL.total_text_bytes;

fn payload(length: usize) -> TestResult<Box<serde_json::value::RawValue>> {
    let inner = length.checked_sub(2).ok_or("JSON payload too short")?;
    let mut text = String::new();
    text.try_reserve_exact(length)?;
    text.push('"');
    text.extend(std::iter::repeat_n('x', inner));
    text.push('"');
    Ok(serde_json::value::RawValue::from_string(text)?)
}

fn install(
    directory: &Path,
    raw: &serde_json::value::RawValue,
) -> TestResult<SchoolClosureDescriptor> {
    let reference = AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(raw.get().as_bytes())))?,
        bytes: u64::try_from(raw.get().len())?,
    };
    let object = super::super::objects::install(&raw, directory, 4)?;
    Ok(SchoolClosureDescriptor { object, reference })
}

#[derive(Deserialize)]
struct Chunks {
    chunks: Vec<String>,
}

fn remove_chunks(directory: &Path, descriptor: &SchoolClosureDescriptor) -> TestResult {
    let bytes = proof_objects::read(directory, &descriptor.object, 32 * 1024)?;
    let inventory: Chunks = serde_json::from_slice(&bytes)?;
    for digest in inventory.chunks {
        std::fs::remove_file(directory.join(digest))?;
    }
    Ok(())
}

fn refuses_before_chunk_io(directory: &Path, descriptor: &SchoolClosureDescriptor) -> TestResult {
    remove_chunks(directory, descriptor)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        load_school(descriptor, directory, &mut grant),
        Err(ReportError::Store(StoreError::Refused { .. }))
    ));
    Ok(())
}

#[test]
fn oversized_reference_refuses_before_chunk_io() -> TestResult {
    let directory = tempfile::tempdir()?;
    let raw = payload(32)?;
    let mut descriptor = install(directory.path(), &raw)?;
    descriptor.reference.bytes = u64::try_from(LIMIT.checked_add(1).ok_or("limit overflow")?)?;
    refuses_before_chunk_io(directory.path(), &descriptor)
}

#[test]
fn oversized_payload_descriptor_refuses_before_chunk_io() -> TestResult {
    let directory = tempfile::tempdir()?;
    let raw = payload(LIMIT.checked_add(1).ok_or("limit overflow")?)?;
    let mut descriptor = install(directory.path(), &raw)?;
    descriptor.reference.bytes = 1;
    refuses_before_chunk_io(directory.path(), &descriptor)
}

#[test]
fn mismatched_reference_length_refuses_before_chunk_io() -> TestResult {
    let directory = tempfile::tempdir()?;
    let raw = payload(32)?;
    let mut descriptor = install(directory.path(), &raw)?;
    descriptor.reference.bytes = descriptor
        .reference
        .bytes
        .checked_sub(1)
        .ok_or("empty payload")?;
    refuses_before_chunk_io(directory.path(), &descriptor)
}

#[test]
fn exact_school_payload_boundary_preserves_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let raw = payload(LIMIT)?;
    let descriptor = install(directory.path(), &raw)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let loaded = load_school(&descriptor, directory.path(), &mut grant)?;
    check!(eq; loaded.bytes.as_slice(), raw.get().as_bytes());
    Ok(())
}
