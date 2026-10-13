use super::*;

#[test]
fn retained_reader_reports_each_missing_occurrence_content_manifest_and_chunk() -> TestResult {
    let (_dir, store, capture, reference) = fixture(b"original")?;
    for key in [
        reference.manifest_key().to_string(),
        content_manifest(&capture),
        chunk_key(&capture, 0),
    ] {
        let dir = tempfile::tempdir()?;
        let incomplete = Store::open(dir.path())?;
        copy_except(&store, &incomplete, &key)?;
        match read_retained_acquisition(&incomplete.snapshot(), &reference, budget()?) {
            Err(CaptureWitnessError::Missing { key: found }) => check!(eq; found, key),
            found => return Err(format!("missing {key} gave {found:?}").into()),
        }
        check!(eq; incomplete.journal_payload(OWNED_CAPTURE_PHASE, &key)?, None);
    }
    Ok(())
}

#[test]
fn retained_reader_refuses_unsupported_codec_and_wrong_chunk_counts() -> TestResult {
    for (field, replacement) in [("encoding", json!("gzip")), ("chunk_bytes", json!(1024))] {
        let (_dir, store, _capture, reference) = fixture(b"original")?;
        let mut changed = payload(&store, reference.manifest_key())?;
        changed[field] = replacement;
        replace(&store, reference.manifest_key(), &changed)?;
        match read_retained_acquisition(&store.snapshot(), &reference, budget()?) {
            Err(CaptureWitnessError::Unsupported { key, field, .. }) => {
                check!(eq; key, reference.manifest_key());
                check!(eq; field, "chunk codec");
            }
            found => return Err(format!("unsupported codec gave {found:?}").into()),
        }
    }
    for count in [0, 2, usize::MAX] {
        let (_dir, store, _capture, reference) = fixture(b"original")?;
        let mut changed = payload(&store, reference.manifest_key())?;
        changed["chunks"] = json!(count);
        replace(&store, reference.manifest_key(), &changed)?;
        require_corrupt(
            read_retained_acquisition(&store.snapshot(), &reference, budget()?),
            reference.manifest_key(),
        )?;
    }
    Ok(())
}

#[test]
fn retained_reader_checks_shared_manifest_shape_layout_and_metadata() -> TestResult {
    for (field, replacement) in [
        ("chunks", json!(2)),
        ("chunk_bytes", json!(1024)),
        ("encoding", json!("raw")),
    ] {
        let (_dir, store, capture, reference) = fixture(b"original")?;
        let key = content_manifest(&capture);
        let mut changed = payload(&store, &key)?;
        changed[field] = replacement;
        replace(&store, &key, &changed)?;
        let result = read_retained_acquisition(&store.snapshot(), &reference, budget()?);
        if field == "chunks" {
            require_corrupt(result, &key)?;
        } else {
            match result {
                Err(CaptureWitnessError::Unsupported {
                    key: found, field, ..
                }) => {
                    check!(eq; found, key);
                    check!(eq; field, "chunk codec");
                }
                found => return Err(format!("unsupported shared codec gave {found:?}").into()),
            }
        }
    }
    let (_dir, store, capture, reference) = fixture(b"original")?;
    let key = content_manifest(&capture);
    let mut changed = payload(&store, &key)?;
    changed["capture"]["bytes"] = json!(9);
    replace(&store, &key, &changed)?;
    require_foreign(
        read_retained_acquisition(&store.snapshot(), &reference, budget()?),
        &key,
    )?;
    Ok(())
}

#[test]
fn retained_reader_refuses_wrong_chunk_indices_base64_and_decoded_lengths() -> TestResult {
    for (field, replacement) in [
        ("chunk_index", json!(1)),
        ("raw_base64", json!(STANDARD.encode(b"short"))),
        ("raw_base64", json!(STANDARD.encode(b"origina"))),
        ("raw_base64", json!(STANDARD.encode(b"original!"))),
        ("raw_base64", json!(STANDARD.encode(b"extra byte"))),
        ("raw_base64", json!("!!!!!!!!!!!!")),
        ("raw_base64", json!("b3JpZ2luYWx=")),
    ] {
        let (_dir, store, capture, reference) = fixture(b"original")?;
        let key = chunk_key(&capture, 0);
        let mut changed = payload(&store, &key)?;
        changed[field] = replacement;
        replace(&store, &key, &changed)?;
        require_corrupt(
            read_retained_acquisition(&store.snapshot(), &reference, budget()?),
            &key,
        )?;
        check!(eq; payload(&store, &key)?, changed);
    }
    Ok(())
}

#[test]
fn retained_reader_refuses_tampering_and_reordering_even_when_lengths_match() -> TestResult {
    let mut body = vec![b'a'; CAPTURE_CHUNK_BYTES];
    body.extend_from_slice(&vec![b'b'; CAPTURE_CHUNK_BYTES]);
    let (_dir, store, capture, reference) = fixture(&body)?;
    let first = chunk_key(&capture, 0);
    let second = chunk_key(&capture, 1);
    let mut left = payload(&store, &first)?;
    let mut right = payload(&store, &second)?;
    std::mem::swap(&mut left["raw_base64"], &mut right["raw_base64"]);
    replace(&store, &first, &left)?;
    replace(&store, &second, &right)?;
    require_corrupt(
        read_retained_acquisition(&store.snapshot(), &reference, budget()?),
        reference.manifest_key(),
    )?;
    Ok(())
}

#[test]
fn retained_reader_refuses_unknown_or_missing_capture_metadata_without_fallback() -> TestResult {
    let (_dir, store, _capture, reference) = fixture(b"original")?;
    let mut changed = payload(&store, reference.manifest_key())?;
    changed["capture"]["body"] = json!([111, 114, 105, 103, 105, 110, 97, 108]);
    replace(&store, reference.manifest_key(), &changed)?;
    match read_retained_acquisition(&store.snapshot(), &reference, budget()?) {
        Err(CaptureWitnessError::Unsupported { key, field, .. }) => {
            check!(eq; key, reference.manifest_key());
            check!(eq; field, "capture metadata");
        }
        found => return Err(format!("unexpected inline body shape gave {found:?}").into()),
    }
    let (_dir, store, _capture, reference) = fixture(b"original")?;
    let mut changed = payload(&store, reference.manifest_key())?;
    let capture = changed["capture"].as_object_mut().ok_or("capture object")?;
    check!(capture.remove("fetched_at").is_some());
    replace(&store, reference.manifest_key(), &changed)?;
    require_corrupt(
        read_retained_acquisition(&store.snapshot(), &reference, budget()?),
        reference.manifest_key(),
    )?;
    Ok(())
}

#[test]
fn oversized_source_chunk_uses_the_chunk_journal_envelope_grant() -> TestResult {
    let (_dir, store, capture, reference) = fixture(b"original")?;
    let key = chunk_key(&capture, 0);
    let mut changed = payload(&store, &key)?;
    changed["raw_base64"] = json!("A".repeat(65_536));
    replace(&store, &key, &changed)?;
    match read_retained_acquisition(&store.snapshot(), &reference, budget()?) {
        Err(CaptureWitnessError::Budget {
            key: found,
            needed,
            max,
            what,
        }) => {
            check!(eq; found, key);
            check!(eq; max, 64 * 1024);
            check!(eq; what, "encoded envelope");
            check!(needed > max);
        }
        found => return Err(format!("expected chunk envelope limit, got {found:?}").into()),
    }
    check!(eq; payload(&store, &key)?, changed);
    Ok(())
}
