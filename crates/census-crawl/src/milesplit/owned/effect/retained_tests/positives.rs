use super::*;

#[test]
fn retained_reader_roundtrips_actual_writer_empty_boundary_and_multichunk_bytes() -> TestResult {
    for size in [
        0,
        1,
        CAPTURE_CHUNK_BYTES - 1,
        CAPTURE_CHUNK_BYTES,
        CAPTURE_CHUNK_BYTES + 1,
        CAPTURE_CHUNK_BYTES * 2 + 7,
    ] {
        let body = vec![b'x'; size];
        let (_dir, store, capture, reference) = fixture(&body)?;
        let before = store.journal_keys(OWNED_CAPTURE_PHASE)?;
        let verified = read_retained_acquisition(&store.snapshot(), &reference, budget()?)?;
        check!(eq; verified.body(), body);
        check!(eq; verified.capture().body.as_slice(), body.as_slice());
        check!(eq; verified.capture().bytes, size);
        check!(eq; verified.capture().content_digest, capture.content_digest);
        check!(eq; verified.reference(), &reference);
        check!(eq; verified.capture().fetched_at, capture.fetched_at);
        check!(eq; store.journal_keys(OWNED_CAPTURE_PHASE)?, before);
    }
    Ok(())
}

#[test]
fn retained_reader_reopens_actual_troy_capture_without_changing_writer_bytes() -> TestResult {
    let body = crate::milesplit::troy_fixture::BODY;
    let (dir, store, capture, reference) = fixture(body)?;
    let original = payload(&store, reference.manifest_key())?;
    drop(store);
    let reopened = Store::open(dir.path().join("store"))?;
    let verified = read_retained_acquisition(&reopened.snapshot(), &reference, budget()?)?;
    check!(eq; verified.body(), body);
    check!(eq; verified.capture().url, capture.url);
    check!(eq; verified.capture().response_url, None);
    check!(eq; verified.capture().content_type, Some("application/json".into()));
    check!(eq; payload(&reopened, reference.manifest_key())?, original);
    Ok(())
}

#[test]
fn retained_reader_uses_one_pinned_view_when_current_content_changes() -> TestResult {
    let body = b"original retained bytes";
    let (_dir, store, capture, reference) = fixture(body)?;
    let view = store.snapshot();
    let key = chunk_key(&capture, 0);
    let mut changed = payload(&store, &key)?;
    changed["raw_base64"] = json!(STANDARD.encode(b"corrupt! retained bytes"));
    replace(&store, &key, &changed)?;
    let verified = read_retained_acquisition(&view, &reference, budget()?)?;
    check!(eq; verified.body(), body);
    require_corrupt(
        read_retained_acquisition(&store.snapshot(), &reference, budget()?),
        reference.manifest_key(),
    )?;
    Ok(())
}

#[test]
fn later_occurrence_returns_actual_metadata_while_sharing_older_capture_metadata() -> TestResult {
    let (_dir, store, mut capture, first) = fixture(b"same content")?;
    let shared_key = content_manifest(&capture);
    let old_shared = payload(&store, &shared_key)?;
    let old_chunk = payload(&store, &chunk_key(&capture, 0))?;
    capture.fetched_at = "2026-10-02T00:00:00Z".into();
    capture.response_url = Some(capture.url.clone());
    capture.from_cache = true;
    let second = archive(&store, &capture)?;
    check!(ne; first.manifest_key(), second.manifest_key());
    let verified = read_retained_acquisition(&store.snapshot(), &second, budget()?)?;
    check!(eq; verified.capture().fetched_at, "2026-10-02T00:00:00Z");
    check!(eq; verified.capture().response_url, capture.response_url);
    check!(eq; verified.capture().from_cache, true);
    check!(eq; verified.reference(), &second);
    check!(eq; verified.body(), b"same content");
    check!(eq; payload(&store, &shared_key)?, old_shared);
    check!(eq; payload(&store, &chunk_key(&capture, 0))?, old_chunk);
    let first_read = read_retained_acquisition(&store.snapshot(), &first, budget()?)?;
    check!(eq; first_read.capture().fetched_at, "2026-10-01T23:44:16Z");
    check!(eq; first_read.capture().response_url, None);
    check!(eq; first_read.capture().from_cache, false);
    Ok(())
}

#[test]
fn captured_http_refusal_and_non_json_bytes_are_verified_without_parser_claims() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let capture = sample(&[0xff, 0, 0x80, b'<'], 403)?;
    let reference = archive(&store, &capture)?;
    let verified = read_retained_acquisition(&store.snapshot(), &reference, budget()?)?;
    check!(eq; verified.body(), capture.body.as_slice());
    check!(eq; verified.capture().status, 403);
    check!(eq; store.journal_keys(OWNED_MEET_PHASE)?.len(), 0);
    Ok(())
}

#[test]
fn cache_replay_alone_keeps_original_occurrence_and_does_not_mint_a_reference() -> TestResult {
    let (_dir, store, mut capture, reference) = fixture(b"same original content")?;
    let keys = store.journal_keys(OWNED_CAPTURE_PHASE)?;
    capture.from_cache = true;
    let replay = archive(&store, &capture)?;
    check!(eq; replay, reference);
    check!(eq; store.journal_keys(OWNED_CAPTURE_PHASE)?, keys);
    let verified = read_retained_acquisition(&store.snapshot(), &replay, budget()?)?;
    check!(eq; verified.capture().from_cache, false);
    check!(eq; verified.capture().fetched_at, "2026-10-01T23:44:16Z");
    check!(eq; verified.body(), capture.body.as_slice());
    Ok(())
}

#[test]
fn identical_content_with_new_status_type_and_site_keeps_new_occurrence_metadata() -> TestResult {
    let (_dir, store, original, first) = fixture(b"shared original bytes")?;
    let shared_key = content_manifest(&original);
    let prior_manifest = payload(&store, &shared_key)?;
    let prior_chunk = payload(&store, &chunk_key(&original, 0))?;
    let mut next = original.clone();
    next.fetched_at = "2026-10-03T00:00:00Z".into();
    next.status = 403;
    next.content_type = Some("text/plain".into());
    next.url = next.url.replace("al.milesplit.com", "wi.milesplit.com");
    next.response_url = Some(next.url.replace("wi.milesplit.com", "www.milesplit.com"));
    let later = archive(&store, &next)?;
    check!(ne; later.manifest_key(), first.manifest_key());
    let verified = read_retained_acquisition(&store.snapshot(), &later, budget()?)?;
    check!(eq; verified.capture().url, next.url);
    check!(eq; verified.capture().response_url, next.response_url);
    check!(eq; verified.capture().status, 403);
    check!(eq; verified.capture().content_type, Some("text/plain".into()));
    check!(eq; verified.capture().fetched_at, "2026-10-03T00:00:00Z");
    check!(eq; verified.body(), original.body.as_slice());
    check!(eq; payload(&store, &shared_key)?, prior_manifest);
    check!(eq; payload(&store, &chunk_key(&original, 0))?, prior_chunk);
    let earlier = read_retained_acquisition(&store.snapshot(), &first, budget()?)?;
    check!(eq; earlier.capture().url, original.url);
    check!(eq; earlier.capture().status, 200);
    check!(eq; earlier.capture().content_type, original.content_type);
    Ok(())
}
