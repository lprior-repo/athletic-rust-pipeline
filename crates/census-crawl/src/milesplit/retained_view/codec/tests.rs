use super::*;
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn actual_visitor_predicate_admits_exact_numeric_limits_and_refuses_excess() -> TestResult {
    check!(eq; admit_entry(MAX_ENTRIES - 1, MAX_PAYLOAD + 2048), Ok(()));
    for (index, bytes) in [(MAX_ENTRIES, 1), (0, MAX_PAYLOAD + 2049), (usize::MAX, 0)] {
        check!(eq; admit_entry(index, bytes),
            Err("retained journal entry envelope exceeds its admission budget"));
    }
    Ok(())
}

#[test]
fn both_encoder_writers_use_the_same_checked_numeric_byte_admission() -> TestResult {
    check!(eq; encoding_needed(MAX_BYTES - 1, 1)?, MAX_BYTES);
    check!(eq; encoding_needed(MAX_BYTES, 1).err().ok_or("excess admitted")?.to_string(),
        "retained journal encoding exceeds byte budget");
    check!(eq; encoding_needed(usize::MAX, 1).err().ok_or("overflow admitted")?.to_string(),
        "retained journal size overflow");
    let bundle = RetainedJournalBundle {
        version: 1,
        store_identity: StoreIdentity::parse(&"a".repeat(64))?,
        snapshot_sequence: 9,
        entries: Vec::new(),
    };
    let canonical = serde_json::to_vec(&bundle)?;
    let mut actual = LimitedBytes::default();
    canonical
        .chunks(7)
        .try_for_each(|chunk| actual.write_all(chunk))?;
    check!(eq; actual.finish(), canonical);
    check!(eq; bundle.encode()?, canonical);
    check!(eq; canonical_digest(&bundle)?, format!("{:x}", Sha256::digest(&canonical)));
    Ok(())
}

#[test]
fn oversized_raw_entry_reaches_actual_serde_mapping_before_typed_decode() -> TestResult {
    let entry = format!("\"{}\"", "x".repeat(MAX_PAYLOAD + 2048));
    let bytes = format!(
        "{{\"version\":1,\"store_identity\":\"{}\",\"snapshot_sequence\":9,\"entries\":[{entry}]}}",
        "a".repeat(64)
    );
    let error = serde_json::from_str::<RetainedJournalBundle>(&bytes)
        .map_err(encoding)
        .err()
        .ok_or("oversized entry admitted")?;
    match error {
        StoreError::Json { detail, source } => {
            check!(eq; detail, "retained source journal bundle");
            check!(source.to_string().starts_with(
                "retained journal entry envelope exceeds its admission budget at line 1 column "
            ));
        }
        other => return Err(format!("unexpected error: {other:?}").into()),
    }
    Ok(())
}

const PHASE: &str = crate::milesplit::OWNED_CAPTURE_PHASE;

fn refused<T>(result: StoreResult<T>, expected: &str) -> TestResult {
    match result.err().ok_or("unexpected success")? {
        StoreError::Refused { detail } => check!(eq; detail, expected),
        other => return Err(format!("unexpected error: {other:?}").into()),
    }
    Ok(())
}

fn present(value: Value, grant: usize) -> RecordedPayload {
    RecordedPayload::Present {
        value,
        minimum_successful_read_budget: grant,
    }
}

fn bundle(payload: RecordedPayload) -> TestResult<RetainedJournalBundle> {
    Ok(RetainedJournalBundle {
        version: 1,
        store_identity: StoreIdentity::parse(&"a".repeat(64))?,
        snapshot_sequence: 7,
        entries: vec![Entry {
            phase: PHASE.into(),
            key: "synthetic/a".into(),
            payload,
        }],
    })
}

fn frozen(bundle: &RetainedJournalBundle) -> StoreResult<FrozenRetainedJournal> {
    let bytes = bundle.encode()?;
    FrozenRetainedJournal::from_descriptor(
        &crate::net::cache::content_digest(&bytes),
        &"a".repeat(64),
        7,
        &bytes,
    )
}

#[test]
fn serialized_recorded_payload_boundary_includes_wrapper_overhead() -> TestResult {
    let overhead = serde_json::to_vec(&present(Value::String(String::new()), MAX_PAYLOAD))?.len();
    let mut value = bundle(present(
        Value::String("x".repeat(MAX_PAYLOAD - overhead)),
        MAX_PAYLOAD,
    ))?;
    check!(eq; serde_json::to_vec(&value.entries[0].payload)?.len(), MAX_PAYLOAD);
    frozen(&value)?;
    value.entries[0].payload = present(
        Value::String("x".repeat(MAX_PAYLOAD - overhead + 1)),
        MAX_PAYLOAD,
    );
    check!(matches!(
        frozen(&value),
        Err(StoreError::BudgetExceeded {
            what: "frozen retained journal payload bytes",
            needed: 4_194_305,
            available: 4_194_304
        })
    ));
    Ok(())
}

#[test]
fn frozen_keeps_anchor_lineage_key_order_phase_and_canonical_refusals() -> TestResult {
    let mut value = bundle(RecordedPayload::Absent)?;
    value.entries.push(Entry {
        phase: PHASE.into(),
        key: "synthetic/a".into(),
        payload: RecordedPayload::Absent,
    });
    refused(
        frozen(&value),
        "frozen retained journal keys are duplicate or unsorted",
    )?;
    value.entries[0].key = "synthetic/z".into();
    refused(
        frozen(&value),
        "frozen retained journal keys are duplicate or unsorted",
    )?;
    value.entries.pop();
    for key in [String::new(), "x".repeat(513), "bad\nkey".into()] {
        value.entries[0].key = key;
        refused(frozen(&value), "unsupported retained source journal key")?;
    }
    value.entries[0].key = "valid".into();
    value.entries[0].phase = "unsupported".into();
    refused(frozen(&value), "unsupported retained source journal key")?;
    value.entries[0].phase = PHASE.into();
    let bytes = value.encode()?;
    let digest = crate::net::cache::content_digest(&bytes);
    refused(
        FrozenRetainedJournal::from_descriptor(&"f".repeat(64), &"a".repeat(64), 7, &bytes),
        "frozen retained journal differs from anchored descriptor",
    )?;
    for (identity, sequence) in [("b".repeat(64), 7), ("a".repeat(64), 8)] {
        refused(
            FrozenRetainedJournal::from_descriptor(&digest, &identity, sequence, &bytes),
            "frozen retained journal has foreign lineage or noncanonical encoding",
        )?;
    }
    refused(
        FrozenRetainedJournal::from_descriptor(&digest, &"a".repeat(64), 0, &bytes),
        "frozen retained journal differs from anchored descriptor",
    )?;
    let mut padded = vec![b' '];
    padded.extend_from_slice(&bytes);
    refused(
        FrozenRetainedJournal::from_descriptor(
            &crate::net::cache::content_digest(&padded),
            &"a".repeat(64),
            7,
            &padded,
        ),
        "frozen retained journal has foreign lineage or noncanonical encoding",
    )?;
    value.version = 2;
    refused(
        frozen(&value),
        "frozen retained journal has foreign lineage or noncanonical encoding",
    )?;
    value.version = 1;
    value.snapshot_sequence = 0;
    refused(
        frozen(&value),
        "frozen retained journal has foreign lineage or noncanonical encoding",
    )?;
    Ok(())
}
