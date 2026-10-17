use super::*;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentInventoryV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use census_store::{
    replay_frozen_evidence_subject_attachment_references_v1,
    FrozenEvidenceSubjectAttachmentBundleV1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[test]
fn native_genuine_documentary_closure_loss_or_tamper_refuses_without_replacing_publication(
) -> TestResult {
    let fixture = Fixture::open()?;
    let objects = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let altered_output = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    inspect(&dataset, &fixture.expected)?;
    let root = output.path().join("publication");
    let authentic = publish(&fixture, &dataset, &root)?;
    let authenticated = publication::verify_published(&authentic)?.generation_digest;
    let authentic_pointer = std::fs::read_link(root.join("current"))?;
    let copied_root = altered_output.path().join("publication");
    let workbook = fixture::copy_publication(&authentic, &copied_root)?;
    let pointer = std::fs::read_link(copied_root.join("current"))?;
    let generation = workbook.parent().ok_or("generation")?;
    let proofs = generation.join("frozen-input.json.proofs");
    let closure = attachment_object(&dataset, objects.path())?;
    let closure_path = proofs.join(&closure);
    let original = std::fs::read(&closure_path)?;
    let manifest = std::fs::read(generation.join("manifest.json"))?;
    let workbook_bytes = std::fs::read(&workbook)?;
    let frozen_bytes = std::fs::read(generation.join("frozen-input.json"))?;
    std::fs::remove_file(&closure_path)?;
    refuse(&fixture, &workbook, &copied_root, &pointer)?;
    std::fs::write(&closure_path, &original)?;
    publication::verify_published(&workbook)?;
    let mut altered = original.clone();
    let byte = altered.first_mut().ok_or("closure descriptor")?;
    *byte ^= 1;
    std::fs::write(&closure_path, altered)?;
    refuse(&fixture, &workbook, &copied_root, &pointer)?;
    std::fs::write(&closure_path, &original)?;
    let (replacement, chunk) = omitted_history_descriptor(&dataset, &proofs, &original)?;
    check!(replacement != original);
    std::fs::write(&closure_path, replacement)?;
    let refusal = crate::export::proof_objects::read(&proofs, &closure, 32 * 1024);
    check!(
        matches!(refusal, Err(crate::report::ReportError::ArtifactCorrupt { path, .. }) if path == closure_path)
    );
    refuse(&fixture, &workbook, &copied_root, &pointer)?;
    check!(eq; std::fs::read(generation.join("manifest.json"))?, manifest);
    check!(eq; std::fs::read(&workbook)?, workbook_bytes);
    check!(eq; std::fs::read(generation.join("frozen-input.json"))?, frozen_bytes);
    std::fs::write(&closure_path, original)?;
    std::fs::remove_file(proofs.join(chunk))?;
    check!(eq; publication::verify_published(&workbook)?.generation_digest, authenticated);
    check!(eq; publication::verify_published(&authentic)?.generation_digest, authenticated);
    check!(eq; std::fs::read_link(root.join("current"))?, authentic_pointer);
    fixture.require_original_unchanged()?;
    fixture.close()
}

fn refuse(fixture: &Fixture, workbook: &Path, root: &Path, pointer: &Path) -> TestResult {
    check!(publication::verify_published(workbook).is_err());
    check!(publication::recover_publication(&fixture.store, root).is_err());
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Descriptor {
    version: u32,
    bytes: usize,
    sha256: String,
    chunks: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Archive {
    schema: String,
    store: String,
    snapshot_sequence: u64,
    public_head: Head,
    inventory: AttachmentInventoryV1,
    prefixes: Vec<Prefix>,
}

#[derive(Serialize, Deserialize)]
struct Head {
    sequence: u64,
    digest: String,
}

#[derive(Serialize, Deserialize)]
struct Prefix {
    kind: String,
    root: Option<Value>,
    physical_rows: u64,
    encoded_bytes: u64,
    rows: Vec<Value>,
}

fn omitted_history_descriptor(
    dataset: &ExportDataset,
    directory: &Path,
    original: &[u8],
) -> TestResult<(Vec<u8>, String)> {
    let bytes = closure_bytes(directory, original)?;
    let mut archive: Archive = serde_json::from_slice(&bytes)?;
    check!(eq; archive.inventory.records, 2);
    check!(archive.prefixes.len() > 2);
    archive.inventory = AttachmentInventoryV1::empty()?;
    archive.prefixes = ["attachment", "receipts"]
        .into_iter()
        .map(|kind| Prefix {
            kind: kind.into(),
            root: None,
            physical_rows: 0,
            encoded_bytes: 0,
            rows: Vec::new(),
        })
        .collect();
    let omitted = serde_json::to_vec(&archive)?;
    let reference = AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&hash(&omitted))?,
        bytes: u64::try_from(omitted.len())?,
    };
    let mut grant = AttachmentReplayGrantV1::new();
    let bundle = FrozenEvidenceSubjectAttachmentBundleV1::from_owned_bytes(
        omitted.clone(),
        reference,
        &mut grant,
    )?;
    let accepted = dataset.verified_public_identities()?.accepted_snapshot();
    let caller_trusted =
        replay_frozen_evidence_subject_attachment_references_v1(&bundle, accepted, &mut grant)?;
    check!(eq; caller_trusted.inventory().records, 0);
    check!(eq; caller_trusted.accepted_public_head(), accepted.head());
    check!(eq; caller_trusted.snapshot_sequence(), accepted.snapshot_sequence());
    let chunk = hash(&omitted);
    check!(!directory.join(&chunk).exists());
    check!(eq; crate::export::proof_objects::install(directory, &omitted)?, chunk);
    let descriptor = Descriptor {
        version: 3,
        bytes: omitted.len(),
        sha256: hash(&omitted),
        chunks: vec![chunk.clone()],
    };
    Ok((serde_json::to_vec(&descriptor)?, chunk))
}

fn closure_bytes(directory: &Path, original: &[u8]) -> TestResult<Vec<u8>> {
    const CHUNK: usize = 4 * 1024 * 1024;
    let descriptor: Descriptor = serde_json::from_slice(original)?;
    check!(eq; descriptor.version, 3);
    check!(descriptor.bytes > 0 && descriptor.bytes <= 512 * 1024 * 1024);
    check!(descriptor.chunks.len() <= 128);
    check!(eq; descriptor.chunks.len(), descriptor.bytes.div_ceil(CHUNK));
    check!(eq; serde_json::to_vec(&descriptor)?, original);
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(descriptor.bytes)?;
    for chunk in &descriptor.chunks {
        let remaining = descriptor
            .bytes
            .checked_sub(bytes.len())
            .ok_or("closure chunk overflow")?;
        let part = crate::export::proof_objects::read(directory, chunk, u64::try_from(CHUNK)?)?;
        check!(eq; part.len(), remaining.min(CHUNK));
        bytes.extend_from_slice(&part);
    }
    check!(eq; bytes.len(), descriptor.bytes);
    check!(eq; hash(&bytes), descriptor.sha256);
    Ok(bytes)
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
