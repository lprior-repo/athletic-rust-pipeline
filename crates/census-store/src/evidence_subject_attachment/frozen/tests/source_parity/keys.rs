use super::*;
use crate::evidence_subject_attachment::frozen::archive;
use serde_json::Value;

fn adopt(bytes: Vec<u8>) -> TestResult<Bundle> {
    let expected = reference(&bytes)?;
    Ok(Bundle::from_owned_bytes(
        bytes,
        expected,
        &mut AttachmentReplayGrantV1::new(),
    )?)
}

fn sized_key(group: usize, length: usize) -> TestResult<Bundle> {
    let fixture = Fixture::new()?;
    match group {
        0 => put(&fixture.store, "row/a", b"null")?,
        1 => {
            fixture
                .store
                .receipts
                .insert([PREFIX, b"a"].concat(), b"null")?;
        }
        _ => return Err("invalid key fixture group".into()),
    }
    let original = capture_rows(&fixture.store.snapshot())?;
    let mut archive = archive::read(original.bytes(), &mut AttachmentReplayGrantV1::new())?;
    let group = archive.prefixes.get_mut(group).ok_or("key group absent")?;
    let row = group.rows.first_mut().ok_or("key row absent")?;
    row.key.resize(length, b'a');
    group.encoded_bytes = u64::try_from(
        row.key
            .len()
            .checked_add(row.value.get().len())
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    adopt(serde_json::to_vec(&archive)?)
}

#[test]
fn decoded_journal_and_receipt_keys_admit_exact_limits_before_one_over_refusal() -> TestResult {
    for (group, maximum, before_decode) in [(0, 4101_usize, 5_usize), (1, 512, 6)] {
        let exact = sized_key(group, maximum)?;
        FrozenSource::read(&exact, &mut AttachmentReplayGrantV1::new())?;
        let mut grant = AttachmentReplayGrantV1::new();
        grant.charge_steps(
            8192_usize
                .checked_sub(before_decode)
                .ok_or(StoreError::CounterOverflow)?,
        )?;
        check!(matches!(
            FrozenSource::read(&exact, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
                EvidenceSubjectAttachmentError::Capacity {
                    resource: "aggregate traversal steps",
                    needed: 8193,
                    available: 8192,
                }
            )))
        ));
        let over = sized_key(
            group,
            maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?,
        )?;
        let mut grant = AttachmentReplayGrantV1::new();
        grant.charge_steps(
            8192_usize
                .checked_sub(before_decode)
                .ok_or(StoreError::CounterOverflow)?,
        )?;
        let needed = u64::try_from(
            maximum
                .checked_add(1)
                .and_then(|n| n.checked_mul(2))
                .ok_or(StoreError::CounterOverflow)?,
        )?;
        let available = u64::try_from(maximum.checked_mul(2).ok_or(StoreError::CounterOverflow)?)?;
        check!(
            matches!(FrozenSource::read(&over, &mut grant), Err(StoreError::BudgetExceeded {
            what: "frozen archive encoded key", needed: actual_needed, available: actual_available,
        }) if actual_needed == needed && actual_available == available)
        );
        check!(matches!(
            grant.charge_steps(0),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn archive_key_spelling_requires_nonempty_even_lowercase_hex_before_decoding() -> TestResult {
    let original = sized_key(0, 32)?;
    for spelling in ["", "0", "AA", "gg", "0G", "é0"] {
        let mut value: Value = serde_json::from_slice(original.bytes())?;
        *value
            .pointer_mut("/prefixes/0/rows/0/key")
            .ok_or("encoded key absent")? = Value::String(spelling.into());
        let bad = adopt(serde_json::to_vec(&value)?)?;
        super::admission::refused(&bad, "frozen archive key spelling")?;
    }
    Ok(())
}

#[test]
fn oversized_encoded_key_refuses_before_invalid_hex_or_decode_work() -> TestResult {
    let original = sized_key(1, 512)?;
    let mut value: Value = serde_json::from_slice(original.bytes())?;
    *value
        .pointer_mut("/prefixes/1/rows/0/key")
        .ok_or("receipt key absent")? = Value::String("G".repeat(1026));
    let bad = adopt(serde_json::to_vec(&value)?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(8186)?;
    check!(matches!(
        FrozenSource::read(&bad, &mut grant),
        Err(StoreError::BudgetExceeded {
            what: "frozen archive encoded key",
            needed: 1026,
            available: 1024,
        })
    ));
    check!(matches!(
        grant.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn duplicate_and_unknown_outer_metadata_refuse_before_archive_issuance() -> TestResult {
    let fixture = Fixture::new()?;
    let original = capture_rows(&fixture.store.snapshot())?;
    let text = std::str::from_utf8(original.bytes())?;
    for extra in ["\"snapshot_sequence\":0,", "\"unrecognized\":null,"] {
        let tail = text.strip_prefix('{').ok_or("archive opening absent")?;
        let bad = adopt(format!("{{{extra}{tail}").into_bytes())?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(
            matches!(FrozenSource::read(&bad, &mut grant), Err(StoreError::Json { detail, source })
            if detail == "frozen attachment archive header" && source.is_data())
        );
        check!(matches!(
            grant.charge_steps(0),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn positional_outer_metadata_cannot_replace_the_canonical_named_archive() -> TestResult {
    let fixture = Fixture::new()?;
    let original = capture_rows(&fixture.store.snapshot())?;
    let object: Value = serde_json::from_slice(original.bytes())?;
    let values = [
        "schema",
        "store",
        "snapshot_sequence",
        "public_head",
        "inventory",
        "prefixes",
    ]
    .into_iter()
    .map(|key| object.get(key).cloned().ok_or("outer field absent"))
    .collect::<Result<Vec<_>, _>>()?;
    let positional = adopt(serde_json::to_vec(&values)?)?;
    super::admission::refused(&positional, "noncanonical frozen attachment archive")
}
