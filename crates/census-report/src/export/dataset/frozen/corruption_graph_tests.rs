use super::super::inventory::StreamInventory;
use super::super::stream::StreamKind;
use super::corruption_fixtures::{
    artifact_corrupt, check_retained_pointer, chunk_bytes, chunk_descriptor, fixture, object_bytes,
    object_path, patch_first, refused_dataset, retained_inventory, rewrite_chunk, Repack,
};
use super::{contains, TestResult};

#[test]
fn frozen_corruption_object_overwritten_in_place_is_refused() -> TestResult {
    let fixture = fixture("corruption:in-place")?;
    let descriptor = chunk_descriptor(&fixture.root, StreamKind::Athletes, 0)?;
    let path = object_path(&fixture.root, &descriptor.sha256);
    check!(eq;
        object_bytes(&fixture.root, &descriptor.sha256)?,
        chunk_bytes(&fixture.root, &descriptor)?
    );
    let forged = b"{\"ordinal\":0}\n";
    std::fs::write(&path, forged)?;
    let error = refused_dataset(&fixture.root, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("does not hash to its name"),
        "{error}"
    );
    check!(eq;
        std::fs::read(&path)?,
        forged.to_vec(),
        "the overwritten object is retained"
    );
    check_retained_pointer(&fixture.root)?;
    Ok(())
}

#[test]
fn frozen_corruption_descriptor_pointing_at_a_foreign_chunk_is_refused() -> TestResult {
    let fixture = fixture("corruption:foreign-chunk")?;
    let foreign = chunk_descriptor(&fixture.root, StreamKind::Schools, 0)?;
    let mut repack = Repack::of(&fixture.root);
    {
        let descriptor = repack
            .descriptors(StreamKind::Athletes)?
            .get_mut(0)
            .ok_or("a frozen chunk descriptor")?;
        descriptor.sha256 = foreign.sha256.clone();
        descriptor.bytes = foreign.bytes;
    }
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(error.to_string().contains("corrupt frozen line"), "{error}");
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    check!(eq;
        descriptor.sha256,
        foreign.sha256,
        "the foreign reference is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_rehashed_frame_is_refused_by_the_lineage_digest() -> TestResult {
    let fixture = fixture("corruption:rehashed-frame")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        patch_first(bytes, b"\"canonical_name\":\"Ada", b'B')
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error
            .to_string()
            .contains("do not reproduce the lineage input digest"),
        "{error}"
    );
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    check!(
        contains(&chunk_bytes(&tampered, &descriptor)?, "AdB Runner"),
        "the smuggled payload is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_inventory_row_total_is_refused() -> TestResult {
    let fixture = fixture("corruption:row-total")?;
    let honest = Repack::of(&fixture.root).page(StreamKind::Athletes)?;
    let declared = honest
        .rows()
        .checked_add(1)
        .ok_or("a row total overflowed")?;
    let mut repack = Repack::of(&fixture.root);
    repack.patch_inventory(StreamKind::Athletes, move |value| {
        let rows = value.get_mut("rows").ok_or("inventory rows")?;
        *rows = serde_json::json!(declared);
        Ok(())
    })?;
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("descriptors account for"),
        "{error}"
    );
    check!(
        error
            .to_string()
            .contains(&format!("against the declared {declared}")),
        "{error}"
    );
    let retained: serde_json::Value =
        serde_json::from_slice(&retained_inventory(&tampered, StreamKind::Athletes)?)?;
    check!(eq;
        retained.get("rows").and_then(|rows| rows.as_u64()),
        Some(declared),
        "the corrupt inventory is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_manifest_reference_totals_are_refused() -> TestResult {
    let fixture = fixture("corruption:reference-total")?;
    let honest = Repack::of(&fixture.root).page(StreamKind::Athletes)?;
    let declared = honest
        .rows()
        .checked_add(1)
        .ok_or("a row total overflowed")?;
    let mut repack = Repack::of(&fixture.root);
    repack.keep_reference(StreamKind::Athletes);
    repack.reference(StreamKind::Athletes)?.rows = declared;
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error
            .to_string()
            .contains("inventory totals disagree with the manifest reference"),
        "{error}"
    );
    let reference = tampered
        .manifest()
        .stream(StreamKind::Athletes)
        .ok_or("a frozen manifest stream")?;
    check!(eq;
        reference.rows,
        declared,
        "the stale reference total is retained"
    );
    check!(eq;
        retained_inventory(&tampered, StreamKind::Athletes)?,
        retained_inventory(&fixture.root, StreamKind::Athletes)?,
        "the honest inventory object is untouched"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_descriptor_byte_length_is_refused() -> TestResult {
    let fixture = fixture("corruption:chunk-length")?;
    let declared = Repack::of(&fixture.root)
        .page(StreamKind::Athletes)?
        .bytes();
    let mut repack = Repack::of(&fixture.root);
    repack.patch_inventory(StreamKind::Athletes, |value| {
        let chunk = value
            .get_mut("chunks")
            .and_then(|chunks| chunks.as_array_mut())
            .and_then(|chunks| chunks.get_mut(0))
            .ok_or("a frozen chunk descriptor")?;
        let bytes = chunk.get_mut("bytes").ok_or("a chunk byte length")?;
        let current = bytes.as_u64().ok_or("a chunk byte length")?;
        *bytes = serde_json::json!(current.checked_add(1).ok_or("a byte length overflowed")?);
        Ok(())
    })?;
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("descriptors account for"),
        "{error}"
    );
    check!(
        error
            .to_string()
            .contains(&format!("against the declared {declared}")),
        "{error}"
    );
    check!(
        StreamInventory::decode(&retained_inventory(&tampered, StreamKind::Athletes)?).is_err(),
        "the corrupt inventory is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_descriptor_row_count_below_the_frames_is_refused() -> TestResult {
    let fixture = fixture("corruption:row-count")?;
    let mut repack = Repack::of(&fixture.root);
    {
        let page = repack.stage_page(StreamKind::Athletes)?;
        page.rows = 1;
        let descriptor = page.chunks.get_mut(0).ok_or("a frozen chunk descriptor")?;
        descriptor.rows = 1;
    }
    let tampered = repack.finish()?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error
            .to_string()
            .contains("holds a different row count than its descriptor"),
        "{error}"
    );
    let retained = StreamInventory::decode(&retained_inventory(&tampered, StreamKind::Athletes)?)?;
    check!(eq; retained.rows(), 1, "the corrupt row count is retained");
    let descriptor = retained
        .chunks()
        .first()
        .ok_or("a frozen chunk descriptor")?;
    check!(eq; descriptor.rows, 1);
    check_retained_pointer(&tampered)?;
    Ok(())
}
