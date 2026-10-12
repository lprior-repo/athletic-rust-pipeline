use super::super::super::codec::{CODEC_REVISION, INVENTORY_CODEC_ID};
use super::super::super::inventory::{ArtifactRef, ChunkDescriptor};
use super::super::super::references::ManifestStream;
use super::super::super::{generation, objects, root, verify, FrozenRoot};
use super::{Repack, StreamInventory, StreamKind, TestResult};
use std::collections::{BTreeMap, BTreeSet};

impl Repack {
    pub(crate) fn of(root: &FrozenRoot) -> Self {
        Self {
            directory: root.directory().to_path_buf(),
            objects_dir: objects::directory(root.directory()),
            attempt: root.attempt().to_string(),
            manifest: root.manifest().clone(),
            pages: BTreeMap::new(),
            keep: BTreeSet::new(),
        }
    }

    pub(crate) fn page(&self, kind: StreamKind) -> TestResult<StreamInventory> {
        let reference = self
            .manifest
            .stream(kind)
            .ok_or("a frozen manifest carries every stream")?;
        Ok(verify::page(&self.objects_dir, reference)?)
    }

    pub(crate) fn chunk(&self, kind: StreamKind, index: usize) -> TestResult<Vec<u8>> {
        let page = self.page(kind)?;
        let descriptor = page
            .chunks()
            .get(index)
            .ok_or("a frozen chunk descriptor")?;
        Ok(objects::read(
            &self.objects_dir,
            &descriptor.sha256,
            objects::CHUNK_CEILING,
        )?)
    }

    fn staged(&mut self, kind: StreamKind) -> TestResult<&mut StreamInventory> {
        if !self.pages.contains_key(&kind.index()) {
            let page = self.page(kind)?;
            self.pages.insert(kind.index(), page);
        }
        self.pages
            .get_mut(&kind.index())
            .ok_or_else(|| "a staged stream page".into())
    }

    pub(crate) fn descriptors(
        &mut self,
        kind: StreamKind,
    ) -> TestResult<&mut Vec<ChunkDescriptor>> {
        Ok(&mut self.staged(kind)?.chunks)
    }

    pub(crate) fn stage_page(&mut self, kind: StreamKind) -> TestResult<&mut StreamInventory> {
        self.staged(kind)
    }

    pub(crate) fn reference(&mut self, kind: StreamKind) -> TestResult<&mut ManifestStream> {
        self.manifest
            .streams
            .get_mut(kind.index())
            .ok_or_else(|| "a manifest stream slot".into())
    }

    pub(crate) fn keep_reference(&mut self, kind: StreamKind) {
        self.keep.insert(kind.index());
    }

    pub(crate) fn put_chunk(&mut self, kind: StreamKind, index: usize, bytes: &[u8]) -> TestResult {
        let digest = objects::install(&self.objects_dir, bytes)?;
        let length = u64::try_from(bytes.len())?;
        let descriptor = self
            .descriptors(kind)?
            .get_mut(index)
            .ok_or("a frozen chunk descriptor")?;
        descriptor.sha256 = digest;
        descriptor.bytes = length;
        Ok(())
    }

    pub(crate) fn adopt(&mut self, donor: &FrozenRoot, kind: StreamKind) -> TestResult {
        let donor_dir = objects::directory(donor.directory());
        let reference = donor
            .manifest()
            .stream(kind)
            .ok_or("a donor manifest carries every stream")?;
        let page = verify::page(&donor_dir, reference)?;
        for descriptor in page.chunks() {
            let bytes = objects::read(&donor_dir, &descriptor.sha256, objects::CHUNK_CEILING)?;
            check!(eq;
                objects::install(&self.objects_dir, &bytes)?,
                descriptor.sha256,
                "a grafted chunk keeps its digest"
            );
        }
        self.pages.insert(kind.index(), page);
        Ok(())
    }

    pub(crate) fn adopt_digest(&mut self, donor: &FrozenRoot) -> TestResult {
        let digest = donor.manifest().lineage.input_digest.clone();
        self.manifest.legacy_logical_digest = Some(digest.clone());
        self.manifest.lineage.input_digest = digest;
        self.manifest.lineage.input_generation = generation(&self.manifest.lineage)?;
        Ok(())
    }

    pub(crate) fn patch_inventory(
        &mut self,
        kind: StreamKind,
        patch: impl FnOnce(&mut serde_json::Value) -> TestResult,
    ) -> TestResult {
        let digest = self.reference(kind)?.inventory.digest.clone();
        let bytes = objects::read(&self.objects_dir, &digest, objects::PAGE_CEILING)?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
        patch(&mut value)?;
        let encoded = serde_json::to_vec(&value)?;
        let patched = objects::install(&self.objects_dir, &encoded)?;
        self.keep_reference(kind);
        self.reference(kind)?.inventory = ArtifactRef {
            digest: patched,
            bytes: u64::try_from(encoded.len())?,
            codec_id: INVENTORY_CODEC_ID.to_string(),
            codec_revision: CODEC_REVISION,
        };
        Ok(())
    }

    pub(crate) fn finish(self) -> TestResult<FrozenRoot> {
        let Self {
            directory,
            objects_dir,
            attempt,
            manifest,
            pages,
            keep,
        } = self;
        let mut manifest = manifest;
        for (index, page) in pages {
            let kind = StreamKind::ALL
                .get(index)
                .copied()
                .ok_or("a staged stream kind")?;
            if keep.contains(&index) {
                continue;
            }
            let (rows, bytes) = totals(page.chunks())?;
            let inventory = StreamInventory::new(kind, page.chunks().to_vec(), rows, bytes)?;
            let encoded = inventory.encode()?;
            let reference = ManifestStream::of(&inventory, &encoded)?;
            check!(eq;
                objects::install(&objects_dir, &encoded)?,
                reference.inventory.digest
            );
            let slot = manifest
                .streams
                .get_mut(kind.index())
                .ok_or("a manifest stream slot")?;
            *slot = reference;
        }
        let encoded = manifest.encode()?;
        let digest = objects::install(&objects_dir, &encoded)?;
        root::write_retained(
            &directory,
            &root::Retained {
                attempt: attempt.clone(),
                manifest: digest.clone(),
            },
        )?;
        let bytes = root::payload_bytes_of(&manifest)?;
        Ok(FrozenRoot::new(directory, manifest, digest, attempt, bytes))
    }
}

fn totals(chunks: &[ChunkDescriptor]) -> TestResult<(u64, u64)> {
    let mut rows: u64 = 0;
    let mut bytes: u64 = 0;
    for descriptor in chunks {
        rows = rows
            .checked_add(descriptor.rows)
            .ok_or("a chunk row total overflowed")?;
        bytes = bytes
            .checked_add(descriptor.bytes)
            .ok_or("a chunk byte total overflowed")?;
    }
    Ok((rows, bytes))
}
