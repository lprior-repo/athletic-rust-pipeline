use super::super::stream::StreamKind;
use super::corruption_fixtures::{
    artifact_corrupt, check_retained_pointer, chunk_bytes, chunk_descriptor, fixture,
    frame_entity_id, line_end, line_start, patch_first, refused_dataset, rewrite_chunk,
    set_line_ordinal,
};
use super::{contains, TestResult};

#[test]
fn frozen_corruption_truncated_frame_is_refused() -> TestResult {
    let fixture = fixture("corruption:truncated-frame")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        let end = bytes.len().saturating_sub(1);
        bytes.truncate(end);
        Ok(())
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error
            .to_string()
            .contains("must end with exactly one newline"),
        "{error}"
    );
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    let retained = chunk_bytes(&tampered, &descriptor)?;
    check!(
        ne;
        retained.last().copied(),
        Some(b'\n'),
        "the truncated frame is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_truncated_json_frame_is_refused() -> TestResult {
    let fixture = fixture("corruption:truncated-json")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        let start = line_start(bytes, 1)?;
        let end = line_end(bytes, 1)?;
        let head = bytes.get(..start).ok_or("a frame head")?.to_vec();
        let tail = bytes.get(end..).ok_or("a frame tail")?;
        let mut rebuilt = head;
        rebuilt.extend_from_slice(b"{\"ordinal\":1,\"entity_id\":\"ath_cut\"");
        rebuilt.extend_from_slice(tail);
        *bytes = rebuilt;
        Ok(())
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(error.to_string().contains("corrupt frozen line"), "{error}");
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    check!(
        contains(&chunk_bytes(&tampered, &descriptor)?, "ath_cut"),
        "the truncated frame is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_malformed_json_frame_is_refused() -> TestResult {
    let fixture = fixture("corruption:malformed-json")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        patch_first(bytes, b"\"payload\":", b',')
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(error.to_string().contains("corrupt frozen line"), "{error}");
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    check!(
        contains(&chunk_bytes(&tampered, &descriptor)?, "\"payload\","),
        "the malformed frame is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_frame_without_the_pinned_ordinal_is_refused() -> TestResult {
    let fixture = fixture("corruption:pinned-ordinal")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        patch_first(bytes, b"{\"ordinal\":", b'[')
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("must begin with the pinned"),
        "{error}"
    );
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    check!(
        contains(&chunk_bytes(&tampered, &descriptor)?, "{\"ordinal\"["),
        "the unpinned frame is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_blank_line_is_refused() -> TestResult {
    let fixture = fixture("corruption:blank-line")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        let insert = line_end(bytes, 0)?.checked_add(1).ok_or("a line end")?;
        bytes.insert(insert, b'\n');
        Ok(())
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("must not hold blank lines"),
        "{error}"
    );
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    check!(
        contains(&chunk_bytes(&tampered, &descriptor)?, "\n\n"),
        "the blank line is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_bad_utf8_frame_is_refused() -> TestResult {
    let fixture = fixture("corruption:bad-utf8")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        let identity = frame_entity_id(bytes, 1)?;
        patch_first(bytes, identity.as_bytes(), 0xFF)
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(error.to_string().contains("must be valid utf-8"), "{error}");
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    let retained = chunk_bytes(&tampered, &descriptor)?;
    check!(
        std::str::from_utf8(&retained).is_err(),
        "the invalid frame is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_ordinal_gap_frame_is_refused() -> TestResult {
    let fixture = fixture("corruption:ordinal-gap")?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        set_line_ordinal(bytes, 1, b'5')
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("expected ordinal 1 but read 5"),
        "{error}"
    );
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    check!(
        contains(&chunk_bytes(&tampered, &descriptor)?, "{\"ordinal\":5"),
        "the gapped ordinal is retained"
    );
    check_retained_pointer(&tampered)?;
    Ok(())
}

#[test]
fn frozen_corruption_foreign_frame_identity_is_refused() -> TestResult {
    let fixture = fixture("corruption:foreign-identity")?;
    let honest = chunk_descriptor(&fixture.root, StreamKind::Athletes, 0)?;
    let before = frame_entity_id(&chunk_bytes(&fixture.root, &honest)?, 1)?;
    let tampered = rewrite_chunk(&fixture.root, StreamKind::Athletes, 0, |bytes| {
        let identity = frame_entity_id(bytes, 1)?;
        let last = identity
            .as_bytes()
            .last()
            .copied()
            .ok_or("an entity id is not empty")?;
        let replacement = if last == b'z' { b'y' } else { b'z' };
        patch_first(bytes, identity.as_bytes(), replacement)
    })?;
    let error = refused_dataset(&tampered, fixture.store())?;
    artifact_corrupt(&error)?;
    check!(
        error.to_string().contains("does not match entity_id"),
        "{error}"
    );
    let descriptor = chunk_descriptor(&tampered, StreamKind::Athletes, 0)?;
    let after = frame_entity_id(&chunk_bytes(&tampered, &descriptor)?, 1)?;
    check!(ne; after, before, "the foreign identity is retained");
    check_retained_pointer(&tampered)?;
    Ok(())
}
