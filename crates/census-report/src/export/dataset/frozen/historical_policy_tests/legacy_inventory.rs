use super::super::fixtures::TestResult;
use super::super::inventory::{ArtifactRef, StreamInventory};
use super::super::manifest::FrozenManifest;
use super::super::objects;
use census_domain::model::serialized_bytes;
use std::path::Path;

pub(in crate::export::dataset::frozen) fn downgrade(
    directory: &Path,
    manifest: &mut FrozenManifest,
) -> TestResult {
    let objects_dir = objects::directory(directory);
    for reference in &mut manifest.streams {
        let encoded = objects::read(
            &objects_dir,
            &reference.inventory.digest,
            objects::PAGE_CEILING,
        )?;
        let mut page = StreamInventory::decode(&encoded)?;
        page.codec = "census-frozen-inventory-v2".into();
        page.codec_revision = 2;
        for descriptor in &mut page.chunks {
            descriptor.codec_revision = 2;
        }
        let encoded = serialized_bytes(&page)?;
        let digest = objects::install(&objects_dir, &encoded)?;
        check!(eq; objects::read(&objects_dir, &digest, objects::PAGE_CEILING)?, encoded);
        reference.inventory = ArtifactRef {
            digest,
            bytes: u64::try_from(encoded.len())?,
            codec_id: "census-frozen-inventory-v2".into(),
            codec_revision: 2,
        };
    }
    manifest.codec = "census-frozen-jsonl-v2".into();
    manifest.codec_revision = 2;
    manifest.input_token.protocol = 2;
    Ok(())
}

pub(in crate::export::dataset::frozen) fn inventory(
    directory: &Path,
    digest: &str,
    manifest: &FrozenManifest,
) -> TestResult<Vec<String>> {
    let objects_dir = objects::directory(directory);
    let mut inventory = vec![digest.to_string()];
    for reference in &manifest.streams {
        check!(eq; reference.inventory.codec_id, "census-frozen-inventory-v2");
        check!(eq; reference.inventory.codec_revision, 2);
        let encoded = objects::read(
            &objects_dir,
            &reference.inventory.digest,
            objects::PAGE_CEILING,
        )?;
        check!(eq; u64::try_from(encoded.len())?, reference.inventory.bytes);
        let page: StreamInventory = serde_json::from_slice(&encoded)?;
        check!(eq; page.codec, "census-frozen-inventory-v2");
        check!(eq; page.codec_revision, 2);
        check!(eq; page.stream, reference.stream);
        check!(eq; page.rows, reference.rows);
        check!(eq; page.bytes, reference.bytes);
        inventory.push(reference.inventory.digest.clone());
        for descriptor in page.chunks {
            check!(eq; descriptor.codec_revision, 2);
            let chunk = objects::read(&objects_dir, &descriptor.sha256, objects::CHUNK_CEILING)?;
            check!(eq; u64::try_from(chunk.len())?, descriptor.bytes);
            inventory.push(descriptor.sha256);
        }
    }
    Ok(inventory)
}
