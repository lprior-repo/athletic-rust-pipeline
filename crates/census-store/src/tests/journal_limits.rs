use super::*;

#[test]
fn journal_roundtrips_resume_keys() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(
        "milesplit_rosters",
        "wi:52649",
        &serde_json::json!({"athletes": 59}),
    )?;
    store.journal_done(
        "milesplit_rosters",
        "wi:52650",
        &serde_json::json!({"athletes": 10}),
    )?;
    let keys = store.journal_keys("milesplit_rosters")?;
    if !keys.contains("wi:52649") {
        return Err(format!("missing first resume key: {keys:?}").into());
    }
    if !keys.contains("wi:52650") {
        return Err(format!("missing second resume key: {keys:?}").into());
    }
    {
        let (left, right) = (&store.journal_payloads("milesplit_rosters")?.len(), &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    store.journal_done(
        "other_phase",
        "wi:52649",
        &serde_json::json!({"athletes": 1}),
    )?;
    {
        let (left, right) = (&store.journal_keys("milesplit_rosters")?, &keys);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_journal_key_past_its_ceiling_is_refused_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let key = "k".repeat(MAX_JOURNAL_KEY_BYTES + 1);
    match store.journal_done("wiaa_schools", &key, &serde_json::json!({"rows": 1})) {
        Err(StoreError::JournalTooLarge {
            what,
            phase,
            max,
            bytes,
            ..
        }) => {
            {
                let (left, right) = (&what, &"key");
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            {
                let (left, right) = (&phase, &"wiaa_schools");
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            {
                let (left, right) = (&max, &MAX_JOURNAL_KEY_BYTES);
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            if bytes <= max {
                return Err(
                    format!("oversize key must exceed bound: bytes={bytes} max={max}").into(),
                );
            }
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("expected the journal key ceiling to refuse the entry".into()),
    }
    let keys = store.journal_keys("wiaa_schools")?;
    if !keys.is_empty() {
        return Err(format!("expected empty keys: {keys:?}").into());
    }
    let payloads = store.journal_payloads("wiaa_schools")?;
    if !payloads.is_empty() {
        return Err(format!("expected empty payloads: {payloads:?}").into());
    }
    Ok(())
}

#[test]
fn a_journal_value_past_its_ceiling_is_chunked_and_reassembled() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done("ihsa_schools", "kept", &serde_json::json!({"rows": 1}))?;
    let oversized = "v".repeat(MAX_JOURNAL_VALUE_BYTES + 1);
    store.journal_done("ihsa_schools", "chunked", &oversized)?;
    let keys = store.journal_keys("ihsa_schools")?;
    if !keys.contains("kept") {
        return Err(format!("kept receipt missing: {keys:?}").into());
    }
    if !keys.contains("chunked") {
        return Err(format!("chunked receipt missing: {keys:?}").into());
    }
    let kept = store.journal_payload("ihsa_schools", "kept")?;
    let Some(kept) = kept else {
        return Err("kept entry vanished from the chunked store".into());
    };
    if kept.get("rows") != Some(&serde_json::json!(1)) {
        return Err("kept entry corrupted by chunking".into());
    }
    let chunked = store.journal_payload("ihsa_schools", "chunked")?;
    if chunked.as_ref() != Some(&serde_json::json!(oversized)) {
        return Err("chunked entry did not reassemble byte-identically".into());
    }
    let payloads = store.journal_payloads("ihsa_schools")?;
    if payloads.len() != 2 {
        return Err(format!("expected 2 payloads, got {}", payloads.len()).into());
    }
    Ok(())
}

#[test]
fn a_1mb_journal_payload_roundtrips_byte_identically() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let meetpro_size = 1_094_892;
    let payload = serde_json::json!({"html": "x".repeat(meetpro_size)});
    store.journal_done("meetpro_partial", "big", &payload)?;
    let keys = store.journal_keys("meetpro_partial")?;
    if !keys.contains("big") {
        return Err(format!("missing key: {keys:?}").into());
    }
    if keys.len() != 1 {
        return Err(format!("expected exactly 1 key, got {}: {:?}", keys.len(), keys).into());
    }
    let read = store.journal_payload("meetpro_partial", "big")?;
    let Some(read) = read else {
        return Err("big payload round-trip returned None".into());
    };
    if read != payload {
        return Err("big payload did not round-trip byte-identically".into());
    }
    Ok(())
}

#[test]
fn mixed_chunked_and_single_journal_entries_list_logical_keys_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done("mixed", "small", &serde_json::json!({"n": 1}))?;
    let big = "b".repeat(MAX_JOURNAL_VALUE_BYTES + 1);
    store.journal_done("mixed", "large", &big)?;
    store.journal_done("mixed", "another_small", &serde_json::json!({"n": 2}))?;
    let keys = store.journal_keys("mixed")?;
    let expected: Vec<&str> = vec!["small", "large", "another_small"];
    if keys.len() != 3 {
        return Err(format!("expected 3 keys, got {}: {:?}", keys.len(), keys).into());
    }
    for expected_key in expected {
        if !keys.contains(expected_key) {
            return Err(format!("missing key {expected_key} in {keys:?}").into());
        }
    }
    let payloads = store.journal_payloads("mixed")?;
    if payloads.len() != 3 {
        return Err(format!("expected 3 payloads, got {}", payloads.len()).into());
    }
    Ok(())
}

#[test]
fn a_payload_far_past_the_ceiling_is_chunked_within_every_physical_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done("refuse", "before", &serde_json::json!({"n": 1}))?;
    let gigantic = "g".repeat(MAX_JOURNAL_VALUE_BYTES * 10);
    store.journal_done("refuse", "gigantic", &gigantic)?;
    let keys = store.journal_keys("refuse")?;
    if !keys.contains("before") || !keys.contains("gigantic") {
        return Err(format!("chunked keys missing: {keys:?}").into());
    }
    let payload = store.journal_payload("refuse", "gigantic")?;
    if payload.as_ref() != Some(&serde_json::json!(gigantic)) {
        return Err("gigantic payload did not reassemble byte-identically".into());
    }
    let physical = store.journal_physical_values("refuse")?;
    for bytes in &physical {
        if *bytes > MAX_JOURNAL_VALUE_BYTES {
            return Err(format!(
                "physical journal value {bytes} exceeds the {MAX_JOURNAL_VALUE_BYTES} ceiling"
            )
            .into());
        }
    }
    if physical.len() < 3 {
        return Err(format!("expected chunked physical values, got {}", physical.len()).into());
    }
    {
        let (left, right) = (&store.journal_payloads("refuse")?.len(), &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}
