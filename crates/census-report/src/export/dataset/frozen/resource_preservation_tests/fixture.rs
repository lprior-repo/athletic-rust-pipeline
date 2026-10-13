use super::super::codec::{
    count, CHUNK_BYTES_LIMIT, LINE_BYTES_LIMIT, MAX_INVENTORY_ROOTS, MAX_JSON_DEPTH,
    MAX_ROOT_TEXT_BYTES,
};
use super::super::fixtures::TestResult;
use super::super::inventory::{ArtifactRef, StreamInventory};
use super::super::manifest::FrozenManifest;
use super::super::root::{self, Retained};
use super::super::stream::StreamKind;
use super::super::{objects, verify, FrozenRoot};
use super::version_records::{install_record, RetainedState};
use crate::report::{ExportResourceError, ReportError};
use census_domain::model::{serialized_bytes, serialized_digest};
use census_store::Store;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResourceCase {
    ManifestText,
    ManifestDepth,
    ManifestRoots,
    InventoryChunk,
    InventoryDepth,
    CursorLine,
    CursorDepth,
    CursorLineCleanup,
}

impl ResourceCase {
    pub(super) const ALL: [Self; 8] = [
        Self::ManifestText,
        Self::ManifestDepth,
        Self::ManifestRoots,
        Self::InventoryChunk,
        Self::InventoryDepth,
        Self::CursorLine,
        Self::CursorDepth,
        Self::CursorLineCleanup,
    ];

    fn expected(self) -> TestResult<ExportResourceError> {
        Ok(match self {
            Self::ManifestText => ExportResourceError::RowTooLarge {
                what: "manifest store root",
                bytes: count(
                    MAX_ROOT_TEXT_BYTES
                        .checked_add(1)
                        .ok_or("root size overflow")?,
                )?,
                max: count(MAX_ROOT_TEXT_BYTES)?,
            },
            Self::ManifestDepth | Self::InventoryDepth | Self::CursorDepth => {
                ExportResourceError::BudgetExceeded {
                    what: "frozen json depth",
                    needed: MAX_JSON_DEPTH.checked_add(1).ok_or("depth overflow")?,
                    available: MAX_JSON_DEPTH,
                }
            }
            Self::ManifestRoots => ExportResourceError::NeedsPartition {
                what: "manifest inventory roots",
                bytes: count(
                    MAX_INVENTORY_ROOTS
                        .checked_add(1)
                        .ok_or("root count overflow")?,
                )?,
                max: count(MAX_INVENTORY_ROOTS)?,
            },
            Self::InventoryChunk => ExportResourceError::RowTooLarge {
                what: "frozen chunk",
                bytes: CHUNK_BYTES_LIMIT
                    .checked_add(1)
                    .ok_or("chunk size overflow")?,
                max: CHUNK_BYTES_LIMIT,
            },
            Self::CursorLine | Self::CursorLineCleanup => ExportResourceError::RowTooLarge {
                what: "frozen line including its newline",
                bytes: LINE_BYTES_LIMIT
                    .checked_add(1)
                    .ok_or("line size overflow")?,
                max: LINE_BYTES_LIMIT,
            },
        })
    }
}

pub(super) struct Rewritten {
    pub(super) manifest: FrozenManifest,
    pub(super) digest: String,
    pub(super) inventory: Vec<String>,
    pub(super) bytes: u64,
}

pub(super) fn install(
    store: &Store,
    current: &FrozenRoot,
    case: ResourceCase,
    state: RetainedState,
) -> TestResult<Retained> {
    let rewritten = rewrite(store, current, case)?;
    let attempt = serialized_digest(&(
        "resource-preservation-fixture",
        format!("{case:?}:{state:?}"),
        &rewritten.digest,
    ))?;
    let retained = Retained {
        attempt,
        manifest: rewritten.digest,
    };
    if let Some(record) = install_record(
        store,
        &retained.attempt,
        rewritten.bytes,
        &rewritten.inventory,
        state,
    )? {
        verify::check_record(&rewritten.manifest, &record, &rewritten.inventory)?;
    }
    root::write_retained(current.directory(), &retained)?;
    Ok(retained)
}

pub(super) fn rewrite(
    store: &Store,
    current: &FrozenRoot,
    case: ResourceCase,
) -> TestResult<Rewritten> {
    let mut manifest = current.manifest().clone();
    let mut replacements = Vec::new();
    change_page(current.directory(), &mut manifest, case, &mut replacements)?;
    match case {
        ResourceCase::ManifestText => {
            manifest.lineage.store_root = "r".repeat(
                MAX_ROOT_TEXT_BYTES
                    .checked_add(1)
                    .ok_or("fixture root size overflow")?,
            );
        }
        ResourceCase::ManifestRoots => {
            let fingerprint = manifest
                .source_fingerprints
                .first()
                .ok_or("fingerprint missing")?
                .clone();
            let extra = MAX_INVENTORY_ROOTS
                .checked_add(1)
                .and_then(|total| total.checked_sub(manifest.streams.len()))
                .ok_or("fixture root count overflow")?;
            manifest.source_fingerprints.resize(extra, fingerprint);
        }
        ResourceCase::ManifestDepth
        | ResourceCase::InventoryChunk
        | ResourceCase::InventoryDepth
        | ResourceCase::CursorLine
        | ResourceCase::CursorDepth
        | ResourceCase::CursorLineCleanup => {}
    }
    let mut encoded = serialized_bytes(&manifest)?;
    if case == ResourceCase::ManifestDepth {
        encoded = with_depth(&encoded)?;
    }
    check!(count(encoded.len())? <= objects::MANIFEST_CEILING);
    let digest = objects::install(&objects::directory(current.directory()), &encoded)?;
    replacements.push((current.manifest_digest().to_string(), digest.clone()));
    let original = store
        .get_input_record(current.attempt())?
        .ok_or("baseline input missing")?;
    let inventory = original
        .chunks()
        .iter()
        .map(|old| replacement(old, &replacements))
        .collect();
    let bytes = root::payload_bytes_of(&manifest)?;
    if case == ResourceCase::CursorLineCleanup {
        std::fs::write(
            current.directory().join("refusals"),
            b"retained cleanup blocker",
        )?;
    }
    Ok(Rewritten {
        manifest,
        digest,
        inventory,
        bytes,
    })
}

fn change_page(
    directory: &Path,
    manifest: &mut FrozenManifest,
    case: ResourceCase,
    replacements: &mut Vec<(String, String)>,
) -> TestResult {
    if matches!(
        case,
        ResourceCase::ManifestText | ResourceCase::ManifestDepth | ResourceCase::ManifestRoots
    ) {
        return Ok(());
    }
    let reference = manifest
        .streams
        .get_mut(StreamKind::Schools.index())
        .ok_or("school stream missing")?;
    let old_page = reference.inventory.digest.clone();
    let objects_dir = objects::directory(directory);
    let encoded = objects::read(&objects_dir, &old_page, objects::PAGE_CEILING)?;
    let mut page = StreamInventory::decode(&encoded)?;
    if case == ResourceCase::InventoryChunk {
        let descriptor = page.chunks.first_mut().ok_or("school chunk missing")?;
        descriptor.bytes = CHUNK_BYTES_LIMIT
            .checked_add(1)
            .ok_or("fixture chunk size overflow")?;
    } else if case != ResourceCase::InventoryDepth {
        change_chunk(&objects_dir, &mut page, case, replacements)?;
        reference.bytes = page.bytes;
    }
    let mut encoded = serialized_bytes(&page)?;
    if case == ResourceCase::InventoryDepth {
        encoded = with_depth(&encoded)?;
    }
    check!(count(encoded.len())? <= objects::PAGE_CEILING);
    reference.inventory = ArtifactRef::of(&encoded)?;
    let digest = objects::install(&objects_dir, &encoded)?;
    check!(eq; reference.inventory.digest, digest);
    replacements.push((old_page, digest));
    Ok(())
}

fn change_chunk(
    directory: &Path,
    page: &mut StreamInventory,
    case: ResourceCase,
    replacements: &mut Vec<(String, String)>,
) -> TestResult {
    let descriptor = page.chunks.first_mut().ok_or("school chunk missing")?;
    let old_chunk = descriptor.sha256.clone();
    let mut encoded = objects::read(directory, &old_chunk, objects::CHUNK_CEILING)?;
    check!(eq; encoded.pop(), Some(b'\n'));
    if case == ResourceCase::CursorDepth {
        encoded = with_depth(&encoded)?;
    } else {
        encoded.resize(usize::try_from(LINE_BYTES_LIMIT)?, b' ');
    }
    encoded.push(b'\n');
    check!(count(encoded.len())? <= objects::CHUNK_CEILING);
    let digest = objects::install(directory, &encoded)?;
    page.bytes = page
        .bytes
        .checked_sub(descriptor.bytes)
        .and_then(|bytes| bytes.checked_add(u64::try_from(encoded.len()).ok()?))
        .ok_or("fixture page size overflow")?;
    descriptor.bytes = count(encoded.len())?;
    descriptor.sha256 = digest.clone();
    replacements.push((old_chunk, digest));
    page.validate()?;
    Ok(())
}

fn with_depth(encoded: &[u8]) -> TestResult<Vec<u8>> {
    let prefix = encoded
        .strip_suffix(b"}")
        .ok_or("fixture object suffix missing")?;
    let depth = usize::try_from(MAX_JSON_DEPTH)?;
    let nested = format!(
        ",\"resource_padding\":{}0{}}}",
        "[".repeat(depth),
        "]".repeat(depth)
    );
    Ok(prefix.iter().copied().chain(nested.bytes()).collect())
}

fn replacement(old: &str, replacements: &[(String, String)]) -> String {
    match replacements.iter().find(|(before, _)| before == old) {
        Some((_, after)) => after.clone(),
        None => old.to_string(),
    }
}

pub(super) fn check_error(error: &ReportError, case: ResourceCase) -> TestResult {
    check!(eq; error.resource(), Some(&case.expected()?));
    check!(eq; error.corruption(), None);
    if case == ResourceCase::CursorLineCleanup {
        check!(matches!(error, ReportError::Cleanup { operation, cleanup }
            if operation.resource().is_some() && cleanup.resource().is_none()));
    } else {
        check!(matches!(error, ReportError::Resource(_)));
    }
    Ok(())
}

pub(super) fn check_cleanup_marker(directory: &Path, case: ResourceCase) -> TestResult {
    if case == ResourceCase::CursorLineCleanup {
        check!(eq; std::fs::read(directory.join("refusals"))?, b"retained cleanup blocker");
    }
    Ok(())
}
