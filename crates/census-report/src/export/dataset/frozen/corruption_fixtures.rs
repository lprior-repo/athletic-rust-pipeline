use super::super::fixtures::{athlete, school, seeded_store, SeededStore, TestResult};
use super::super::inventory::{ChunkDescriptor, StreamInventory};
use super::super::manifest::FrozenManifest;
use super::super::stream::StreamKind;
use super::super::{freeze_v2, objects, root, to_export_dataset, verify, FrozenRoot};
use crate::report::{CorruptionClass, ReportError};
use census_store::{InputRecord, Store, Table};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[path = "corruption_fixtures/repack.rs"]
mod repack;

const ORDINAL_HEAD: &[u8] = b"{\"ordinal\":";
const ENTITY_HEAD: &[u8] = b"\"entity_id\":\"";
const HUGE_ROW_BYTES: usize = 200 * 1024;
const HUGE_ROWS: u32 = 24;

pub(crate) struct Fixture {
    pub(crate) seeded: SeededStore,
    pub(crate) root: FrozenRoot,
}

pub(crate) fn fixture(job: &str) -> TestResult<Fixture> {
    let seeded = seeded_store()?;
    let root = freeze_v2(&seeded.store, job)?;
    Ok(Fixture { seeded, root })
}

pub(crate) fn two_chunk_fixture(job: &str) -> TestResult<Fixture> {
    let seeded = seeded_store()?;
    let anchor = school("Bounded High");
    seeded.store.append(Table::Schools, &anchor)?;
    for index in 0..HUGE_ROWS {
        let mut row = athlete(
            &anchor.id,
            &format!("Huge Runner {index}"),
            &format!("huge-{index}"),
        );
        row.canonical_name = "x".repeat(HUGE_ROW_BYTES);
        seeded.store.append(Table::Athletes, &row)?;
    }
    let root = freeze_v2(&seeded.store, job)?;
    let second = chunk_descriptor(&root, StreamKind::Athletes, 1)?;
    check!(eq;
        second.index,
        1,
        "a missing-chunk fixture needs two sealed chunks"
    );
    Ok(Fixture { seeded, root })
}

impl Fixture {
    pub(crate) fn store(&self) -> &Store {
        &self.seeded.store
    }
}

pub(crate) struct GraftPair {
    pub(crate) seeded: SeededStore,
    pub(crate) base: FrozenRoot,
    pub(crate) donor: FrozenRoot,
}

impl GraftPair {
    pub(crate) fn store(&self) -> &Store {
        &self.seeded.store
    }

    pub(crate) fn base_record(&self) -> TestResult<InputRecord> {
        self.store()
            .get_input_record(self.base.attempt())?
            .ok_or_else(|| "an attempt keeps its durable input record".into())
    }
}

pub(crate) fn graft_pair() -> TestResult<GraftPair> {
    let seeded = seeded_store()?;
    let base = freeze_v2(&seeded.store, "corruption:graft-base")?;
    seeded
        .store
        .append(Table::Schools, &school("Appended High"))?;
    let donor = freeze_v2(&seeded.store, "corruption:graft-donor")?;
    check!(ne; donor.manifest_digest(), base.manifest_digest());
    Ok(GraftPair {
        seeded,
        base,
        donor,
    })
}

pub(crate) fn refused_dataset(root: &FrozenRoot, store: &Store) -> TestResult<ReportError> {
    match to_export_dataset(root, store) {
        Ok(_) => Err("a corrupted frozen root produced a dataset".into()),
        Err(error) => Ok(error),
    }
}

pub(crate) fn refused_record(root: &FrozenRoot, record: &InputRecord) -> TestResult<ReportError> {
    let objects_dir = objects::directory(root.directory());
    match verify::verify_record(
        &objects_dir,
        root.manifest_digest(),
        root.manifest(),
        record,
    ) {
        Ok(()) => Err("a corrupted frozen root verified against its durable record".into()),
        Err(error) => Ok(error),
    }
}

pub(crate) fn artifact_corrupt(error: &ReportError) -> TestResult {
    check!(eq;
        error.corruption(),
        Some(CorruptionClass::ArtifactCorrupt),
        "{error}"
    );
    Ok(())
}

pub(crate) fn chunk_descriptor(
    root: &FrozenRoot,
    kind: StreamKind,
    index: usize,
) -> TestResult<ChunkDescriptor> {
    let reference = root
        .manifest()
        .stream(kind)
        .ok_or("a frozen manifest carries every stream")?;
    let page = verify::page(&objects::directory(root.directory()), reference)?;
    Ok(page
        .chunks()
        .get(index)
        .cloned()
        .ok_or("a frozen chunk descriptor")?)
}

pub(crate) fn chunk_bytes(root: &FrozenRoot, descriptor: &ChunkDescriptor) -> TestResult<Vec<u8>> {
    Ok(objects::read(
        &objects::directory(root.directory()),
        &descriptor.sha256,
        objects::CHUNK_CEILING,
    )?)
}

pub(crate) fn object_bytes(root: &FrozenRoot, digest: &str) -> TestResult<Vec<u8>> {
    Ok(std::fs::read(object_path(root, digest))?)
}

pub(crate) fn object_path(root: &FrozenRoot, digest: &str) -> PathBuf {
    objects::directory(root.directory()).join(digest)
}

pub(crate) fn check_retained_pointer(root: &FrozenRoot) -> TestResult {
    let retained = root::read_retained(root.directory())?
        .ok_or("a corrupt fixture keeps its retained root pointer")?;
    check!(eq;
        retained.manifest,
        root.manifest_digest(),
        "the retained root pointer still names the fixture manifest"
    );
    Ok(())
}

pub(crate) fn retained_inventory(root: &FrozenRoot, kind: StreamKind) -> TestResult<Vec<u8>> {
    let reference = root
        .manifest()
        .stream(kind)
        .ok_or("a frozen manifest carries every stream")?;
    Ok(objects::read(
        &objects::directory(root.directory()),
        &reference.inventory.digest,
        objects::PAGE_CEILING,
    )?)
}

pub(crate) struct Repack {
    directory: PathBuf,
    objects_dir: PathBuf,
    attempt: String,
    manifest: FrozenManifest,
    pages: BTreeMap<usize, StreamInventory>,
    keep: BTreeSet<usize>,
}

pub(crate) fn rewrite_chunk(
    root: &FrozenRoot,
    kind: StreamKind,
    index: usize,
    rewrite: impl FnOnce(&mut Vec<u8>) -> TestResult,
) -> TestResult<FrozenRoot> {
    let mut repack = Repack::of(root);
    let mut bytes = repack.chunk(kind, index)?;
    let before = bytes.clone();
    rewrite(&mut bytes)?;
    check!(ne; bytes, before, "a corruption fixture must change the chunk");
    repack.put_chunk(kind, index, &bytes)?;
    repack.finish()
}

pub(crate) fn set_line_ordinal(bytes: &mut [u8], line: usize, digit: u8) -> TestResult {
    let start = line_start(bytes, line)?;
    let head = start
        .checked_add(ORDINAL_HEAD.len())
        .ok_or("an ordinal offset overflowed")?;
    check!(eq;
        bytes.get(start..head),
        Some(ORDINAL_HEAD),
        "a frozen line pins the ordinal field first"
    );
    let slot = bytes.get_mut(head).ok_or("an ordinal digit")?;
    check!(slot.is_ascii_digit(), "the pinned ordinal holds a digit");
    *slot = digit;
    Ok(())
}

pub(crate) fn frame_entity_id(bytes: &[u8], line: usize) -> TestResult<String> {
    let start = line_start(bytes, line)?;
    let end = line_end(bytes, line)?;
    let frame = bytes.get(start..end).ok_or("a frame escaped the object")?;
    let head = find(frame, ENTITY_HEAD).ok_or("a frame carries its entity id")?;
    let tail = head
        .checked_add(ENTITY_HEAD.len())
        .ok_or("an entity id offset overflowed")?;
    let rest = frame.get(tail..).ok_or("an entity id escaped the frame")?;
    let stop = rest
        .iter()
        .position(|byte| *byte == b'"')
        .ok_or("a frame closes its entity id")?;
    let id = rest.get(..stop).ok_or("an entity id escaped the frame")?;
    Ok(String::from_utf8(id.to_vec())?)
}

pub(crate) fn patch_first(haystack: &mut [u8], needle: &[u8], replacement: u8) -> TestResult {
    let start = find(haystack, needle).ok_or("a fixture needle is missing")?;
    let last = start
        .checked_add(needle.len())
        .and_then(|end| end.checked_sub(1))
        .ok_or("a fixture needle offset overflowed")?;
    let slot = haystack
        .get_mut(last)
        .ok_or("a fixture needle escaped the object")?;
    check!(ne; *slot, replacement, "a fixture patch must change the object");
    *slot = replacement;
    Ok(())
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(crate) fn line_start(bytes: &[u8], line: usize) -> TestResult<usize> {
    let mut start: usize = 0;
    let mut seen: usize = 0;
    for (position, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        if seen == line {
            return Ok(start);
        }
        seen = seen.checked_add(1).ok_or("a line counter overflowed")?;
        start = position.checked_add(1).ok_or("a line offset overflowed")?;
    }
    Err("a chunk line is missing".into())
}

pub(crate) fn line_end(bytes: &[u8], line: usize) -> TestResult<usize> {
    let start = line_start(bytes, line)?;
    let rest = bytes
        .get(start..)
        .ok_or("a fixture line escaped the object")?;
    let stop = rest
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or("a fixture line is unterminated")?;
    Ok(start.checked_add(stop).ok_or("a line offset overflowed")?)
}
