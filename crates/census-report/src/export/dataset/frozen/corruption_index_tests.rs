use super::super::codec::{CODEC_REVISION, INVENTORY_CODEC_ID};
use super::super::inventory::{ArtifactRef, StreamInventory};
use super::super::root;
use super::super::stream::StreamKind;
use super::super::{ensure_current, objects, to_export_dataset, FrozenRoot};
use super::corruption_fixtures::{
    artifact_corrupt, check_retained_pointer, chunk_bytes, chunk_descriptor, fixture, graft_pair,
    object_bytes, object_path, refused_dataset, refused_record, retained_inventory,
    two_chunk_fixture, Repack,
};
use super::TestResult;
use crate::report::ReportError;

#[test]
fn frozen_corruption_extra_chunk_descriptor_is_refused() -> TestResult {
    let fixture = fixture("corruption:extra-chunk")?;
    let mut repack = Repack::of(&fixture.root);
    {
        let page = repack.stage_page(StreamKind::Athletes)?;
        let first = page
            .chunks
            .first()
            .cloned()
            .ok_or("a frozen chunk descriptor")?;
        let mut second = first.clone();
        second.index = first
            .index
            .checked_add(1)
            .ok_or("a chunk index overflowed")?;
        second.first_ordinal = first
            .first_ordinal
            .checked_add(first.rows)
            .ok_or("a chunk ordinal overflowed")?;
        page.chunks.push(second);
    }
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("expected ordinal 2 but read 0"),
        "{error}"
    );
    let retained = StreamInventory::decode(&retained_inventory(&tampered, StreamKind::Athletes)?)?;
    check!(eq;
        retained.chunks().len(),
        2,
        "the extra descriptor is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_missing_chunk_index_gap_is_refused() -> TestResult {
    let fixture = two_chunk_fixture("corruption:missing-chunk")?;
    let mut repack = Repack::of(&fixture.root);
    repack.patch_inventory(StreamKind::Athletes, |value| {
        let chunk = value
            .get_mut("chunks")
            .and_then(|chunks| chunks.as_array_mut())
            .and_then(|chunks| chunks.get_mut(1))
            .ok_or("the second chunk descriptor")?;
        let index = chunk.get_mut("index").ok_or("a chunk index")?;
        *index = serde_json::json!(3);
        Ok(())
    })?;
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error
            .to_string()
            .contains("chunk index 3 appears where 1 was expected"),
        "{error}"
    );
    check!(
        StreamInventory::decode(&retained_inventory(&tampered, StreamKind::Athletes)?).is_err(),
        "the gapped inventory is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_deleted_chunk_object_is_refused() -> TestResult {
    let fixture = two_chunk_fixture("corruption:deleted-object")?;
    let descriptor = chunk_descriptor(&fixture.root, StreamKind::Athletes, 1)?;
    let path = object_path(&fixture.root, &descriptor.sha256);
    std::fs::remove_file(&path)?;
    let error = refused_dataset(&fixture.root, fixture.store())?;
    check!(
        error.corruption().is_none(),
        "a missing object is refused outside the corruption classes: {error}"
    );
    check!(matches!(error, ReportError::Io { .. }), "{error}");
    check!(error.to_string().contains(&descriptor.sha256), "{error}");
    check!(!path.exists(), "the referenced object stays missing");
    check_retained_pointer(&fixture.root)?;
    Ok(())
}

#[test]
fn frozen_corruption_reference_pointing_at_a_chunk_object_is_refused() -> TestResult {
    let fixture = fixture("corruption:wrong-reference")?;
    let descriptor = chunk_descriptor(&fixture.root, StreamKind::Athletes, 0)?;
    let length = u64::try_from(chunk_bytes(&fixture.root, &descriptor)?.len())?;
    let mut repack = Repack::of(&fixture.root);
    repack.keep_reference(StreamKind::Athletes);
    repack.reference(StreamKind::Athletes)?.inventory = ArtifactRef {
        digest: descriptor.sha256.clone(),
        bytes: length,
        codec_id: INVENTORY_CODEC_ID.to_string(),
        codec_revision: CODEC_REVISION,
    };
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("corrupt stream inventory"),
        "{error}"
    );
    let reference = tampered
        .manifest()
        .stream(StreamKind::Athletes)
        .ok_or("a frozen manifest stream")?;
    check!(eq;
        reference.inventory.digest,
        descriptor.sha256,
        "the invalid reference is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_unsupported_manifest_codec_revision_is_refused() -> TestResult {
    let fixture = fixture("corruption:manifest-revision")?;
    let mut manifest = fixture.root.manifest().clone();
    manifest.codec_revision = CODEC_REVISION
        .checked_add(1)
        .ok_or("a codec revision overflowed")?;
    let encoded = serde_json::to_vec(&manifest)?;
    let objects_dir = objects::directory(fixture.root.directory());
    let digest = objects::install(&objects_dir, &encoded)?;
    root::write_retained(
        fixture.root.directory(),
        &root::Retained {
            attempt: fixture.root.attempt().to_string(),
            manifest: digest.clone(),
        },
    )?;
    let tampered = FrozenRoot::new(
        fixture.root.directory().to_path_buf(),
        manifest,
        digest.clone(),
        fixture.root.attempt().to_string(),
        fixture.root.bytes(),
    );
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("unsupported frozen manifest"),
        "{error}"
    );
    check!(error.to_string().contains("revision 3"), "{error}");
    check!(eq;
        object_bytes(&tampered, &digest)?,
        encoded,
        "the refusing manifest is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_non_digest_chunk_reference_is_refused() -> TestResult {
    let fixture = fixture("corruption:non-digest")?;
    let mut repack = Repack::of(&fixture.root);
    repack.patch_inventory(StreamKind::Athletes, |value| {
        let digest = value
            .get_mut("chunks")
            .and_then(|chunks| chunks.as_array_mut())
            .and_then(|chunks| chunks.get_mut(0))
            .and_then(|chunk| chunk.get_mut("sha256"))
            .ok_or("a chunk digest")?;
        *digest = serde_json::json!("not-a-sha256-digest");
        Ok(())
    })?;
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error
            .to_string()
            .contains("is not a lowercase sha256 digest"),
        "{error}"
    );
    check!(
        StreamInventory::decode(&retained_inventory(&tampered, StreamKind::Athletes)?).is_err(),
        "the invalid reference is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_unreferenced_extra_object_is_ignored() -> TestResult {
    let fixture = fixture("corruption:orphan")?;
    let orphan = b"{\"ordinal\":0,\"entity_id\":\"ath_orphan\",\"payload\":{}}\n";
    let objects_dir = objects::directory(fixture.root.directory());
    let digest = objects::install(&objects_dir, orphan)?;
    let dataset = to_export_dataset(&fixture.root, fixture.store())?;
    check!(eq;
        dataset.athletes.len(),
        2,
        "an unreferenced object carries no rows"
    );
    check!(ne;
        digest,
        chunk_descriptor(&fixture.root, StreamKind::Athletes, 0)?.sha256
    );
    check!(eq;
        object_bytes(&fixture.root, &digest)?,
        orphan.to_vec(),
        "the unreferenced object is retained"
    );
    check_retained_pointer(&fixture.root)?;
    Ok(())
}

#[test]
fn frozen_corruption_cross_attempt_graft_is_refused_by_the_retained_record() -> TestResult {
    let pair = graft_pair()?;
    let mut repack = Repack::of(&pair.base);
    repack.adopt(&pair.donor, StreamKind::Schools)?;
    repack.adopt_digest(&pair.donor)?;
    let grafted = repack.finish()?;
    let dataset = to_export_dataset(&grafted, pair.store())?;
    check!(eq;
        dataset.schools.len(),
        2,
        "the artifact verifier materializes the grafted payload"
    );
    let record = pair.base_record()?;
    let error = refused_record(&grafted, &record)?;
    check!(
        error
            .to_string()
            .contains("does not inventory the frozen artifacts"),
        "{error}"
    );
    check!(
        error.corruption().is_none(),
        "the record gate refuses outside the corruption classes: {error}"
    );
    let error = match ensure_current(&dataset, pair.store()) {
        Ok(()) => return Err("a grafted root published changed source evidence".into()),
        Err(error) => error,
    };
    check!(error.to_string().contains("stale or foreign"), "{error}");
    check!(eq;
        record.chunks().first().map(String::as_str),
        Some(pair.base.manifest_digest()),
        "the base record still names the base manifest"
    );
    check!(
        objects::read(
            &objects::directory(pair.base.directory()),
            pair.base.manifest_digest(),
            objects::MANIFEST_CEILING
        )
        .is_ok(),
        "the replaced manifest object is retained"
    );
    check_retained_pointer(&pair.donor)?;
    Ok(())
}
