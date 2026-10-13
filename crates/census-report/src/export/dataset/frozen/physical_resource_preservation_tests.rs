use super::codec::{count, sha256_hex};
use super::fixtures::{seeded_store, TestResult};
use super::inventory::{ArtifactRef, StreamInventory};
use super::root::{self, Retained};
use super::stream::StreamKind;
use super::{cancel_freeze_v2, freeze_v2, objects, FrozenRoot};
use crate::report::{ExportResourceError, ReportError};
use census_domain::model::{serialized_bytes, serialized_digest};
use census_store::Store;
use version_records::{install_record, RetainedState};
use version_snapshot::RetainedEvidence;

#[path = "coverage_tests/version_records.rs"]
mod version_records;
#[allow(clippy::duplicate_mod)]
#[path = "coverage_tests/version_snapshot.rs"]
mod version_snapshot;

#[derive(Debug, Clone, Copy)]
enum Oversize {
    Manifest,
    Page,
    Chunk,
}

impl Oversize {
    fn ceiling(self) -> u64 {
        match self {
            Self::Manifest => objects::MANIFEST_CEILING,
            Self::Page => objects::PAGE_CEILING,
            Self::Chunk => objects::CHUNK_CEILING,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    Resume,
    Cancel,
}

const STATES: [RetainedState; 5] = [
    RetainedState::Missing,
    RetainedState::Requested,
    RetainedState::Freezing,
    RetainedState::Frozen,
    RetainedState::Abandoned,
];

fn padded(mut encoded: Vec<u8>, ceiling: u64) -> TestResult<Vec<u8>> {
    let size = usize::try_from(ceiling.checked_add(1).ok_or("fixture ceiling overflow")?)?;
    let extra = size
        .checked_sub(encoded.len())
        .ok_or("fixture already exceeds ceiling")?;
    encoded.try_reserve(extra)?;
    encoded.resize(size, b' ');
    check!(eq; count(encoded.len())?, ceiling.checked_add(1).ok_or("ceiling overflow")?);
    Ok(encoded)
}

fn install(
    store: &Store,
    current: &FrozenRoot,
    case: Oversize,
    state: RetainedState,
) -> TestResult<Retained> {
    let mut manifest = current.manifest().clone();
    let objects_dir = objects::directory(current.directory());
    let mut replacements = Vec::new();
    if !matches!(case, Oversize::Manifest) {
        let reference = manifest
            .streams
            .get_mut(StreamKind::Schools.index())
            .ok_or("school stream missing")?;
        let old_page = reference.inventory.digest.clone();
        let page_bytes = objects::read(&objects_dir, &old_page, objects::PAGE_CEILING)?;
        let encoded = match case {
            Oversize::Page => padded(page_bytes, case.ceiling())?,
            Oversize::Chunk => {
                let mut page = StreamInventory::decode(&page_bytes)?;
                let descriptor = page.chunks.first_mut().ok_or("school chunk missing")?;
                let old_chunk = descriptor.sha256.clone();
                let chunk = objects::read(&objects_dir, &old_chunk, objects::CHUNK_CEILING)?;
                let oversized = padded(chunk, case.ceiling())?;
                descriptor.sha256 = objects::install(&objects_dir, &oversized)?;
                replacements.push((old_chunk, descriptor.sha256.clone()));
                page.encode()?
            }
            Oversize::Manifest => return Err("manifest fixture cannot replace a page".into()),
        };
        reference.inventory = ArtifactRef::of(&encoded)?;
        let installed = objects::install(&objects_dir, &encoded)?;
        check!(eq; reference.inventory.digest, installed);
        replacements.push((old_page, installed));
    }
    let manifest_bytes = serialized_bytes(&manifest)?;
    let encoded = match case {
        Oversize::Manifest => padded(manifest_bytes, case.ceiling())?,
        Oversize::Page | Oversize::Chunk => manifest_bytes,
    };
    let digest = objects::install(&objects_dir, &encoded)?;
    replacements.push((current.manifest_digest().to_string(), digest.clone()));
    let original = store
        .get_input_record(current.attempt())?
        .ok_or("baseline input missing")?;
    let inventory = original
        .chunks()
        .iter()
        .map(|old| replacement(old, &replacements))
        .collect::<Vec<_>>();
    let retained = Retained {
        attempt: serialized_digest(&(
            "physical-resource-preservation",
            format!("{case:?}:{state:?}"),
            &digest,
        ))?,
        manifest: digest,
    };
    if let Some(record) = install_record(
        store,
        &retained.attempt,
        original.bytes(),
        &inventory,
        state,
    )? {
        super::verify::check_record(&manifest, &record, &inventory)?;
    }
    root::write_retained(current.directory(), &retained)?;
    Ok(retained)
}

fn replacement(old: &str, replacements: &[(String, String)]) -> String {
    match replacements.iter().find(|(before, _)| before == old) {
        Some((_, after)) => after.clone(),
        None => old.to_string(),
    }
}

fn check_error(error: &ReportError, case: Oversize) -> TestResult {
    let expected = ExportResourceError::RowTooLarge {
        what: "frozen object",
        bytes: case
            .ceiling()
            .checked_add(1)
            .ok_or("fixture size overflow")?,
        max: case.ceiling(),
    };
    check!(eq; error.resource(), Some(&expected));
    check!(eq; error.corruption(), None);
    check!(matches!(error, ReportError::Resource(_)));
    Ok(())
}

fn refused(store: &Store, job: &str, case: Oversize, operation: Operation) -> TestResult {
    let error = match operation {
        Operation::Resume => match freeze_v2(store, job) {
            Ok(_) => return Err("physically oversized input was resumed or replaced".into()),
            Err(error) => error,
        },
        Operation::Cancel => match cancel_freeze_v2(store, job) {
            Ok(_) => return Err("physically oversized input was cancelled or swept".into()),
            Err(error) => error,
        },
    };
    check_error(&error, case)
}

fn preserved(case: Oversize, state: RetainedState, operation: Operation) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = format!("physical-resource-preservation:{case:?}:{state:?}:{operation:?}");
    let current = freeze_v2(&store, &job)?;
    let directory = current.directory().to_path_buf();
    let retained = install(&store, &current, case, state)?;
    let before =
        RetainedEvidence::capture(&store, &directory, &[current.attempt(), &retained.attempt])?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&store, &job, case, operation)?;
        before.unchanged(&store, &directory)
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    before.unchanged(&reopened, &directory)?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&reopened, &job, case, operation)?;
        before.unchanged(&reopened, &directory)
    })?;
    check!(eq; root::read_retained(&directory)?, Some(retained));
    Ok(())
}

fn matrix(operation: Operation) -> TestResult {
    [Oversize::Manifest, Oversize::Page, Oversize::Chunk]
        .into_iter()
        .try_for_each(|case| {
            STATES
                .into_iter()
                .try_for_each(|state| preserved(case, state, operation))
        })
}

#[test]
fn physical_resource_preservation_resume_keeps_oversized_objects_in_every_state_after_reopen(
) -> TestResult {
    matrix(Operation::Resume)
}

#[test]
fn physical_resource_preservation_cancel_keeps_oversized_objects_in_every_state_after_reopen(
) -> TestResult {
    matrix(Operation::Cancel)
}

#[test]
fn physical_resource_preservation_exact_ceiling_remains_readable() -> TestResult {
    let directory = tempfile::tempdir()?;
    let size = usize::try_from(objects::MANIFEST_CEILING)?;
    let bytes = vec![b' '; size];
    let digest = objects::install(directory.path(), &bytes)?;
    check!(eq; sha256_hex(&bytes), digest);
    check!(eq; objects::read(directory.path(), &digest, objects::MANIFEST_CEILING)?, bytes);
    Ok(())
}

#[test]
fn physical_resource_preservation_size_refusal_precedes_explicit_descriptor_length_check(
) -> TestResult {
    let seeded = seeded_store()?;
    let job = "physical-resource-preservation:chunk-length";
    let root = freeze_v2(&seeded.store, job)?;
    let directory = root.directory().to_path_buf();
    let retained = install(&seeded.store, &root, Oversize::Chunk, RetainedState::Frozen)?;
    let before = RetainedEvidence::capture(
        &seeded.store,
        &directory,
        &[root.attempt(), &retained.attempt],
    )?;
    refused(&seeded.store, job, Oversize::Chunk, Operation::Resume)?;
    before.unchanged(&seeded.store, &directory)
}

#[test]
fn physical_resource_preservation_bounded_explicit_length_mismatch_remains_corruption() -> TestResult
{
    let seeded = seeded_store()?;
    let root = freeze_v2(
        &seeded.store,
        "physical-resource-preservation:length-corruption",
    )?;
    let directory = objects::directory(root.directory());
    let mut manifest = root.manifest().clone();
    let reference = manifest
        .streams
        .get_mut(StreamKind::Schools.index())
        .ok_or("school stream missing")?;
    let encoded = objects::read(
        &directory,
        &reference.inventory.digest,
        objects::PAGE_CEILING,
    )?;
    let mut page = StreamInventory::decode(&encoded)?;
    let descriptor = page.chunks.first_mut().ok_or("school chunk missing")?;
    let mut chunk = objects::read(&directory, &descriptor.sha256, objects::CHUNK_CEILING)?;
    chunk.push(b' ');
    check!(count(chunk.len())? <= objects::CHUNK_CEILING);
    descriptor.sha256 = objects::install(&directory, &chunk)?;
    let path = directory.join(&descriptor.sha256);
    let encoded = page.encode()?;
    reference.inventory = ArtifactRef::of(&encoded)?;
    let page_digest = objects::install(&directory, &encoded)?;
    check!(eq; reference.inventory.digest, page_digest);
    let encoded = manifest.encode()?;
    let digest = objects::install(&directory, &encoded)?;
    let error: ReportError = match super::verify::verify_root(&directory, &digest, &manifest) {
        Ok(_) => return Err("wrong-length chunk was verified".into()),
        Err(error) => error.into(),
    };
    check!(eq; error.resource(), None);
    check!(
        matches!(error, ReportError::ArtifactCorrupt { path: actual, detail }
        if actual == path && detail == "the frozen chunk length disagrees with its descriptor")
    );
    Ok(())
}
