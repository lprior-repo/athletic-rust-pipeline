use super::*;
use census_store::MAX_JOURNAL_VALUE_BYTES;

fn build_large_payload(size_bytes: usize) -> serde_json::Value {
    let data = "x".repeat(size_bytes);
    serde_json::json!({
        "html": data,
        "meet": "725218",
        "rsid": "1266814",
        "disposition": "partial",
        "capture": {
            "fetched_at": "2026-10-01T23:44:16Z",
            "content_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "bytes": size_bytes,
            "status": 200,
            "content_type": "text/html"
        }
    })
}

#[test]
fn a_partial_payload_past_the_journal_ceiling_replays_without_hitting_the_ceiling() -> TestResult {
    let (_dir, store, _fetcher, _reference) = setup()?;
    let payload = build_large_payload(1_094_892);
    let encoded = serde_json::to_vec(&payload)?;
    if encoded.len() <= MAX_JOURNAL_VALUE_BYTES {
        return Err(format!(
            "payload holds {} bytes, not past the {MAX_JOURNAL_VALUE_BYTES} ceiling",
            encoded.len()
        )
        .into());
    }
    let key = "725218/big/manifest".to_string();
    store.journal_done(super::super::super::RESULT_SET_PHASE, &key, &payload)?;
    let keys = store.journal_keys(super::super::super::RESULT_SET_PHASE)?;
    if !keys.contains(&key) {
        return Err(format!("missing key: {keys:?}").into());
    }
    if keys.len() != 1 {
        return Err(format!("expected exactly 1 key, got {}: {:?}", keys.len(), keys).into());
    }
    let read = store.journal_payload(super::super::super::RESULT_SET_PHASE, &key)?;
    let Some(read) = read else {
        return Err("large payload round-trip returned None".into());
    };
    if read != payload {
        return Err("large payload did not round-trip byte-identically".into());
    }
    let payloads = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
    if payloads.len() != 1 {
        return Err(format!("expected 1 payload, got {}", payloads.len()).into());
    }
    Ok(())
}
