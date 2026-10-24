use super::CanonicalCount;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SEED: &[u8] = b"retained journal counter seed";

fn seeded(bytes: u64) -> CanonicalCount {
    let mut hash = Sha256::new();
    hash.update(SEED);
    CanonicalCount { bytes, hash }
}

fn state(counter: &CanonicalCount, bytes: u64, hashed: &[u8]) -> TestResult {
    check!(eq; counter.bytes(), bytes);
    check!(eq; format!("{:x}", counter.hash.clone().finalize()), format!("{:x}", Sha256::digest(hashed)));
    Ok(())
}

fn refused(result: io::Result<usize>, reason: &str) -> TestResult {
    let error = result.err().ok_or("counter write accepted excess")?;
    check!(eq; error.kind(), io::ErrorKind::Other);
    check!(eq; error.to_string(), reason);
    Ok(())
}

#[test]
fn actual_write_accepts_exact_32_gib_then_refuses_next_byte_without_state_change() -> TestResult {
    let mut counter = seeded(34_359_738_367);
    check!(eq; counter.write(b"x")?, 1);
    let mut expected = SEED.to_vec();
    expected.extend_from_slice(b"x");
    state(&counter, 34_359_738_368, &expected)?;
    refused(counter.write(b"y"), "byte budget exceeded")?;
    state(&counter, 34_359_738_368, &expected)?;
    check!(eq; counter.write(b"")?, 0);
    counter.flush()?;
    state(&counter, 34_359_738_368, &expected)?;
    check!(eq; counter.finish(), (34_359_738_368, format!("{:x}", Sha256::digest(&expected))));
    Ok(())
}

#[test]
fn actual_write_refuses_crossing_ceiling_without_hashing_or_counting_partial_bytes() -> TestResult {
    let mut counter = seeded(34_359_738_367);
    refused(counter.write(b"xy"), "byte budget exceeded")?;
    state(&counter, 34_359_738_367, SEED)?;
    check!(eq; counter.write(b"z")?, 1);
    let mut expected = SEED.to_vec();
    expected.extend_from_slice(b"z");
    check!(eq; counter.finish(), (34_359_738_368, format!("{:x}", Sha256::digest(&expected))));
    Ok(())
}

#[test]
fn write_all_at_ceiling_refuses_and_preserves_seeded_hash_and_count() -> TestResult {
    let mut counter = seeded(34_359_738_368);
    let error = counter
        .write_all(b"next")
        .err()
        .ok_or("write_all accepted excess")?;
    check!(eq; error.kind(), io::ErrorKind::Other);
    check!(eq; error.to_string(), "byte budget exceeded");
    state(&counter, 34_359_738_368, SEED)?;
    refused(counter.write(b"x"), "byte budget exceeded")?;
    state(&counter, 34_359_738_368, SEED)?;
    check!(eq; counter.finish(), (34_359_738_368, format!("{:x}", Sha256::digest(SEED))));
    Ok(())
}

#[test]
fn actual_write_refuses_u64_overflow_without_changing_seeded_hash_or_count() -> TestResult {
    let mut counter = seeded(u64::MAX);
    refused(counter.write(b"x"), "counter overflow")?;
    state(&counter, u64::MAX, SEED)?;
    refused(counter.write(b"yz"), "counter overflow")?;
    state(&counter, u64::MAX, SEED)?;
    check!(eq; counter.finish(), (u64::MAX, format!("{:x}", Sha256::digest(SEED))));
    Ok(())
}
