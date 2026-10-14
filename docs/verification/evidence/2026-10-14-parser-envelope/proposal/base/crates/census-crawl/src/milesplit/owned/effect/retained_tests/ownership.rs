use super::*;

#[test]
fn acquisition_metadata_must_reproduce_requested_occurrence_key() -> TestResult {
    let (_dir, store, _capture, reference) = fixture(b"original")?;
    let mut changed = payload(&store, reference.manifest_key())?;
    changed["capture"]["fetched_at"] = json!("2026-10-02T00:00:00Z");
    replace(&store, reference.manifest_key(), &changed)?;
    require_foreign(
        read_retained_acquisition(&store.snapshot(), &reference, budget()?),
        reference.manifest_key(),
    )?;
    check!(eq; payload(&store, reference.manifest_key())?, changed);
    Ok(())
}

#[test]
fn occurrence_cannot_redirect_to_another_shared_content_owner_or_prefix() -> TestResult {
    for field in ["content_manifest", "chunk_key_prefix"] {
        let (_dir, store, _capture, reference) = fixture(b"original")?;
        let mut changed = payload(&store, reference.manifest_key())?;
        changed[field] = json!(format!("999999/{}", "a".repeat(64)));
        replace(&store, reference.manifest_key(), &changed)?;
        require_foreign(
            read_retained_acquisition(&store.snapshot(), &reference, budget()?),
            reference.manifest_key(),
        )?;
    }
    Ok(())
}

#[test]
fn exact_request_and_response_urls_refuse_foreign_meets_hosts_and_shapes() -> TestResult {
    let original = sample(b"original", 200)?;
    let foreign = original.url.replace("725218", "725219");
    let foreign_host = original
        .url
        .replace("al.milesplit.com", "al.milesplit.com.evil.test");
    let credentials = original.url.replace("https://", "https://user:secret@");
    let alternate_query = original.url.replace("isMeetPro=0", "isMeetPro=1");
    for (request, response) in [
        (foreign, None),
        (foreign_host, None),
        (credentials, None),
        (alternate_query, None),
        (format!("{}#ignored", original.url), None),
        (
            original.url.clone(),
            Some(original.url.replace("725218", "725219")),
        ),
        (
            original.url.clone(),
            Some(original.url.replace("al.milesplit.com", "evil.test")),
        ),
        (
            original.url.clone(),
            Some(original.url.replace("https://", "http://")),
        ),
        (
            original.url.clone(),
            Some(format!("{}#ignored", original.url)),
        ),
    ] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let mut capture = sample(b"original", 200)?;
        capture.url = request;
        capture.response_url = response;
        let reference = archive(&store, &capture)?;
        require_foreign(
            read_retained_acquisition(&store.snapshot(), &reference, budget()?),
            reference.manifest_key(),
        )?;
    }
    Ok(())
}

#[test]
fn repeated_chunk_metadata_binds_immutable_content_but_keeps_actual_occurrence_time() -> TestResult
{
    let (_dir, store, capture, reference) = fixture(b"original")?;
    let key = chunk_key(&capture, 0);
    let original = payload(&store, &key)?;
    for (field, changed_value) in [
        ("content_digest", json!("b".repeat(64))),
        ("bytes", json!(9)),
    ] {
        let mut changed = original.clone();
        changed["capture"][field] = changed_value;
        replace(&store, &key, &changed)?;
        require_foreign(
            read_retained_acquisition(&store.snapshot(), &reference, budget()?),
            &key,
        )?;
    }
    replace(&store, &key, &original)?;
    let verified = read_retained_acquisition(&store.snapshot(), &reference, budget()?)?;
    check!(eq; verified.capture().fetched_at, capture.fetched_at);
    check!(eq; verified.body(), capture.body.as_slice());
    Ok(())
}

#[test]
fn invalid_captured_sha_method_and_http_status_are_corrupt() -> TestResult {
    for (field, value) in [
        ("content_digest", json!("A".repeat(64))),
        ("method", json!("POST")),
        ("status", json!(0)),
    ] {
        let (_dir, store, _capture, reference) = fixture(b"original")?;
        let mut changed = payload(&store, reference.manifest_key())?;
        changed["capture"][field] = value;
        replace(&store, reference.manifest_key(), &changed)?;
        require_corrupt(
            read_retained_acquisition(&store.snapshot(), &reference, budget()?),
            reference.manifest_key(),
        )?;
    }
    Ok(())
}

#[test]
fn shared_manifest_and_chunk_independently_refuse_foreign_response_ownership() -> TestResult {
    for manifest in [true, false] {
        let (_dir, store, capture, reference) = fixture(b"original")?;
        let key = if manifest {
            content_manifest(&capture)
        } else {
            chunk_key(&capture, 0)
        };
        let mut changed = payload(&store, &key)?;
        changed["capture"]["response_url"] = json!(capture.url.replace("725218", "725219"));
        replace(&store, &key, &changed)?;
        require_foreign(
            read_retained_acquisition(&store.snapshot(), &reference, budget()?),
            &key,
        )?;
    }
    Ok(())
}
