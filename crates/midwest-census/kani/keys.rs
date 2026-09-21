//! Kani proof harnesses for the observation-key encoding (`store::keys`) and the store's id bound.
//!
//! A key is `<table>\0<id>\0<sequence:u64 big-endian>` and `split_observation_key` recovers the
//! triple from that fixed-width tail. `kani::any::<String>()` has no `Arbitrary` impl, so the id
//! arrives as a bounded byte array through `String::from_utf8_lossy` - which is also what lets the
//! round trip cover a NUL inside the id: only the *first* NUL (the one after the table name)
//! separates the id, and the sequence is read positionally, never by searching for a NUL.

use crate::store::keys::{observation_id, observation_key, split_observation_key};
use crate::store::{MAX_ID_BYTES, Table};

/// Bound on the symbolic id.
const ID_BYTES: usize = 8;

/// Bound on the raw byte string handed to `split_observation_key`.
const RAW_KEY_BYTES: usize = 24;

fn any_id() -> String {
    let bytes: [u8; ID_BYTES] = kani::any();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Round trip over every table, an arbitrary id and an arbitrary sequence.
#[kani::proof]
#[kani::unwind(48)]
fn check_observation_key_round_trip() {
    let id = any_id();
    let sequence: u64 = kani::any();

    for table in Table::ALL {
        let key = observation_key(table, &id, sequence);
        let (table_back, id_back, sequence_back) =
            split_observation_key(&key).expect("a key we built must split");

        assert_eq!(
            Table::from_wire(table_back),
            Some(table),
            "table did not survive the round trip"
        );
        assert_eq!(id_back, id, "id did not survive the round trip");
        assert_eq!(sequence_back, sequence, "sequence did not survive the round trip");
    }
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

        assert_eq!(Table::from_wire(table_back), Some(table));
        assert_eq!(id_back, id);
        assert_eq!(sequence_back, sequence);
    }
}

/// Sequences whose low byte is zero, and the extremes, round trip.
///
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

        assert_eq!(Table::from_wire(table_back), Some(Table::Schools));
        assert_eq!(id_back, id);
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
