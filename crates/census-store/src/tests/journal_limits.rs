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
fn a_journal_value_past_its_ceiling_leaves_the_phase_as_it_was() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done("ihsa_schools", "kept", &serde_json::json!({"rows": 1}))?;
    let oversized = "v".repeat(MAX_JOURNAL_VALUE_BYTES + 1);
    match store.journal_done("ihsa_schools", "refused", &oversized) {
        Err(StoreError::JournalTooLarge {
            what,
            phase,
            key,
            max,
            bytes,
        }) => {
            {
                let (left, right) = (&what, &"value");
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            {
                let (left, right) = (&phase, &"ihsa_schools");
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            {
                let (left, right) = (&key, &"refused");
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            {
                let (left, right) = (&max, &MAX_JOURNAL_VALUE_BYTES);
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            if bytes <= max {
                return Err(
                    format!("oversize value must exceed bound: bytes={bytes} max={max}").into(),
                );
            }
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("expected the journal value ceiling to refuse the entry".into()),
    }
    let keys = store.journal_keys("ihsa_schools")?;
    if !keys.contains("kept") {
        return Err(format!("kept receipt missing: {keys:?}").into());
    }
    if keys.contains("refused") {
        return Err(format!("refused receipt was written: {keys:?}").into());
    }
    {
        let (left, right) = (&store.journal_payloads("ihsa_schools")?.len(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}
