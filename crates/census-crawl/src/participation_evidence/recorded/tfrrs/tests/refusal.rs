use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

#[test]
fn normal_journal_key_substitution_refuses_checked_holder_and_poisons_replay() -> TestResult {
    let (directory, store) = fixture::fresh()?;
    let (mut recorded, original) = fixture::recorded(&store)?;
    let wrong = "f".repeat(64);
    check!(wrong != original);
    recorded
        .journal
        .first_mut()
        .ok_or("normal retained root")?
        .key = wrong.clone();
    fixture::install(&store, recorded, "tfrrs-key-substitution")?;
    drop(store);
    let store = fixture::reopen(&directory)?;
    let before = fixture::state(&store)?;
    let raw = fixture::raw(&store, &wrong)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        matches!(fixture::present(&store, &wrong, &fixture::locators(), &mut grant),
        Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail }))
        if detail == "recorded TFRRS list context differs from its typed key")
    );
    fixture::poisoned(fixture::present(
        &store,
        &wrong,
        &fixture::locators(),
        &mut grant,
    ))?;
    let mut independent = AttachmentReplayGrantV1::new();
    check!(!fixture::present(
        &store,
        &original,
        &fixture::locators(),
        &mut independent
    )?);
    independent.ensure_available()?;
    check!(eq; fixture::raw(&store, &wrong)?, raw);
    check!(eq; fixture::state(&store)?, before);
    Ok(())
}

#[derive(Clone, Copy)]
enum Tamper {
    Hash,
    Body,
    Length,
}

fn tamper(payload: &mut serde_json::Value, change: Tamper) -> TestResult {
    match change {
        Tamper::Hash => *payload.get_mut("body_sha256").ok_or("body hash")? = "0".repeat(64).into(),
        Tamper::Length => *payload.get_mut("body_bytes").ok_or("body length")? = 1.into(),
        Tamper::Body => {
            let encoded = payload
                .get("body_base64")
                .and_then(serde_json::Value::as_str)
                .ok_or("body encoding")?;
            let mut bytes = STANDARD.decode(encoded)?;
            *bytes.last_mut().ok_or("body final byte")? ^= 1;
            *payload.get_mut("body_base64").ok_or("body encoding")? = STANDARD.encode(bytes).into();
        }
    }
    Ok(())
}

#[test]
fn normal_journal_hash_body_and_length_tamper_refuse_read_only_after_cold_reopen() -> TestResult {
    for (index, change) in [Tamper::Hash, Tamper::Body, Tamper::Length]
        .into_iter()
        .enumerate()
    {
        let (directory, store) = fixture::fresh()?;
        let (mut recorded, key) = fixture::recorded(&store)?;
        tamper(
            &mut recorded
                .journal
                .first_mut()
                .ok_or("normal retained root")?
                .payload,
            change,
        )?;
        fixture::install(&store, recorded, &format!("tfrrs-body-tamper-{index}"))?;
        drop(store);
        let store = fixture::reopen(&directory)?;
        let before = fixture::state(&store)?;
        let raw = fixture::raw(&store, &key)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(fixture::present(&store, &key, &fixture::locators(), &mut grant).is_err());
        fixture::poisoned(fixture::present(
            &store,
            &key,
            &fixture::locators(),
            &mut grant,
        ))?;
        check!(eq; fixture::raw(&store, &key)?, raw);
        check!(eq; fixture::state(&store)?, before);
    }
    Ok(())
}

#[test]
fn cold_locator_mutations_refuse_and_never_refund_or_reuse_a_poisoned_grant() -> TestResult {
    let (directory, store) = fixture::fresh()?;
    let (recorded, key) = fixture::recorded(&store)?;
    fixture::install(&store, recorded, "tfrrs-locator-refusal")?;
    drop(store);
    let store = fixture::reopen(&directory)?;
    let before = fixture::state(&store)?;
    let raw = fixture::raw(&store, &key)?;
    for change in 0..3 {
        let mut requested = fixture::locators();
        let first = requested.first_mut().ok_or("first source locator")?;
        match change {
            0 => first.ordinal += 1,
            1 => first.byte_offset += 1,
            _ => first.byte_length -= 1,
        }
        let mut grant = AttachmentReplayGrantV1::new();
        check!(fixture::present(&store, &key, &requested, &mut grant).is_err());
        let usage = grant.usage();
        check!(usage.encoded_bytes() > 0);
        fixture::poisoned(fixture::present(
            &store,
            &key,
            &fixture::locators(),
            &mut grant,
        ))?;
        check!(eq; grant.usage(), usage);
        check!(eq; fixture::raw(&store, &key)?, raw);
        check!(eq; fixture::state(&store)?, before);
    }
    let mut independent = AttachmentReplayGrantV1::new();
    check!(fixture::present(
        &store,
        &key,
        &fixture::locators(),
        &mut independent
    )?);
    check!(eq; fixture::state(&store)?, before);
    Ok(())
}
