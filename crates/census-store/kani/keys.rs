//! Kani proof harnesses for the observation-key encoding (`store::keys`) and the store's id bound.
//!
//! A key is `<table>\0<id>\0<sequence:u64 big-endian>` and `split_observation_key` recovers the
//! triple from that fixed-width tail. `kani::any::<String>()` has no `Arbitrary` impl, so the id
//! arrives as a bounded byte array through hex encoding — a total, branch-free bijection that
//! keeps every byte visible and never introduces replacement characters, so the round trip is
//! provable for all byte values including NUL.

use crate::keys::{observation_id, observation_key, split_observation_key};
use crate::keys::table_prefix;
use crate::{Table, MAX_ID_BYTES};

/// Bound on the symbolic id.
const ID_BYTES: usize = 8;

/// Bound on the raw byte string handed to `split_observation_key`.
const RAW_KEY_BYTES: usize = 24;
/// Hex lookup table for branch-free byte→char mapping.
const HEX: [u8; 16] = *b"0123456789abcdef";
/// Build the observation key bytes from an id as raw bytes (no String round-trip).
/// This avoids CBMC's opaque `String::as_bytes()` model.
fn build_key(table: Table, id_bytes: &[u8], sequence: u64) -> Vec<u8> {
    let mut out = table_prefix(table);
    out.extend_from_slice(id_bytes);
    out.push(0);
    out.extend_from_slice(&sequence.to_be_bytes());
    out
}

/// Generate a hex-encoded id as raw bytes — total, branch-free bijection.
fn any_id_bytes() -> [u8; ID_BYTES * 2] {
    let bytes: [u8; ID_BYTES] = kani::any();
    let mut out = [0u8; ID_BYTES * 2];
    for i in 0..ID_BYTES {
        out[i * 2] = HEX[(bytes[i] >> 4) as usize];
        out[i * 2 + 1] = HEX[(bytes[i] & 0x0f) as usize];
    }
    out
}

/// Round trip over every table, an arbitrary id and an arbitrary sequence.
#[kani::proof]
#[kani::unwind(48)]
fn check_observation_key_round_trip() {
    let id_bytes = any_id_bytes();
    let sequence: u64 = kani::any();

    for table in Table::ALL {
        let key = build_key(table, &id_bytes, sequence);
        let (table_back, id_back, sequence_back) =
            split_observation_key(&key).expect("a key we built must split");

        assert_eq!(table.file().as_bytes(), table_back, "table did not survive the round trip");
        assert_eq!(id_back, id_bytes.as_slice(), "id did not survive the round trip");
        assert_eq!(sequence_back, sequence, "sequence did not survive the round trip");
    }

    kani::cover!(
        id_bytes.iter().all(|&b| b == 0),
        "all-zero id is reachable"
    );
    kani::cover!(id_bytes.len() == ID_BYTES * 2, "max-length id is reachable");
    kani::cover!(sequence == 0, "zero sequence is reachable");
    kani::cover!(sequence == u64::MAX, "max sequence is reachable");
}

/// A NUL byte inside the id survives the round trip.
#[kani::proof]
#[kani::unwind(48)]
fn check_observation_key_null_byte_id() {
    let id = "id\0with\0null";
    let sequence: u64 = kani::any();

    for table in Table::ALL {
        let key = observation_key(table, id, sequence);
        let (table_back, id_back, sequence_back) =
            split_observation_key(&key).expect("a key we built must split");

        assert_eq!(table.file().as_bytes(), table_back);
        assert_eq!(id_back, id.as_bytes());
        assert_eq!(sequence_back, sequence);
    }
}
/// A parser that searched backwards for a NUL misparsed exactly these, which is why the sequence is
/// the fixed-width tail.
#[kani::proof]
#[kani::unwind(48)]
fn check_observation_key_zero_and_max_sequence() {
    let id = "test_id";

    for sequence in [0_u64, 1, 0x100, 0xffff_ffff, u64::MAX - 1, u64::MAX] {
        let key = observation_key(Table::Schools, id, sequence);
        let (table_back, id_back, sequence_back) =
            split_observation_key(&key).expect("a key we built must split");

        assert_eq!(Table::Schools.file().as_bytes(), table_back);
        assert_eq!(id_back, id.as_bytes());
        assert_eq!(sequence_back, sequence, "sequence was misparsed");
    }
}

/// `split_observation_key` is total on arbitrary bytes, and when it answers, it answered about the
/// fixed-width tail it was actually given.
#[kani::proof]
#[kani::unwind(48)]
fn check_split_key_reads_fixed_width_tail() {
    let bytes: [u8; RAW_KEY_BYTES] = kani::any();

    if let Some((table, id, sequence)) = split_observation_key(&bytes) {
        assert_eq!(
            bytes[RAW_KEY_BYTES - 9],
            0,
            "split accepted a key with no separator before the tail"
        );
        let tail: [u8; 8] = bytes[RAW_KEY_BYTES - 8..]
            .try_into()
            .expect("eight bytes remain after the separator");
        assert_eq!(
            u64::from_be_bytes(tail),
            sequence,
            "the sequence must be the fixed-width big-endian tail"
        );
        assert_eq!(
            table.len() + id.len() + 10,
            RAW_KEY_BYTES,
            "a split key must be table + NUL + id + NUL + 8 bytes"
        );
    }

    kani::cover!(
        bytes[RAW_KEY_BYTES - 9] == 0,
        "bytes with separator before tail are reachable"
    );
    kani::cover!(
        bytes[RAW_KEY_BYTES - 9] != 0,
        "bytes without separator before tail are reachable"
    );
}

/// The id contract `observation_id` enforces on a serialized observation: non-empty, at most
/// `MAX_ID_BYTES`, and carried through verbatim. Fjall asserts keys stay under 64 KiB, so this
/// bound is what keeps an over-long id from reaching the keyspace.
#[kani::proof]
#[kani::unwind(48)]
fn check_observation_id_bounds() {
    for len in [
        0_usize,
        1,
        2,
        MAX_ID_BYTES - 1,
        MAX_ID_BYTES,
        MAX_ID_BYTES + 1,
    ] {
        let id = "a".repeat(len);
        let row = format!(r#"{{"id":"{id}"}}"#);
        let parsed = observation_id(row.as_bytes());

        if (1..=MAX_ID_BYTES).contains(&len) {
            assert_eq!(
                parsed.expect("an in-bound id must parse"),
                id.as_str(),
                "an id of {len} bytes must be accepted verbatim"
            );
        } else {
            assert!(
                parsed.is_err(),
                "an id of {len} bytes must be rejected, not stored"
            );
        }
    }
}
