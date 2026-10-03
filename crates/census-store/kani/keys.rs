
use crate::keys::{observation_id, observation_key, split_observation_key};
use crate::keys::table_prefix;
use crate::{Table, MAX_ID_BYTES};

const ID_BYTES: usize = 8;

const RAW_KEY_BYTES: usize = 24;
const HEX: [u8; 16] = *b"0123456789abcdef";
fn build_key(table: Table, id_bytes: &[u8], sequence: u64) -> Vec<u8> {
    let mut out = table_prefix(table);
    out.extend_from_slice(id_bytes);
    out.push(0);
    out.extend_from_slice(&sequence.to_be_bytes());
    out
}

fn any_id_bytes() -> [u8; ID_BYTES * 2] {
    let bytes: [u8; ID_BYTES] = kani::any();
    let mut out = [0u8; ID_BYTES * 2];
    for i in 0..ID_BYTES {
        out[i * 2] = HEX[(bytes[i] >> 4) as usize];
        out[i * 2 + 1] = HEX[(bytes[i] & 0x0f) as usize];
    }
    out
}

#[kani::proof]
#[kani::unwind(48)]
fn check_observation_key_round_trip() {
    let id_bytes = any_id_bytes();
    let sequence: u64 = kani::any();

    for table in Table::ALL {
        let key = build_key(table, &id_bytes, sequence);
        let parsed = split_observation_key(&key);
        assert_eq!(
            parsed.map(|(table, _, _)| table),
            Some(table.file().as_bytes()),
            "table did not survive the round trip"
        );
        assert_eq!(
            parsed.map(|(_, id, _)| id),
            Some(id_bytes.as_slice()),
            "id did not survive the round trip"
        );
        assert_eq!(
            parsed.map(|(_, _, sequence)| sequence),
            Some(sequence),
            "sequence did not survive the round trip"
        );
    }

    kani::cover!(
        id_bytes.iter().all(|&byte| byte == b'0'),
        "all-zero hexadecimal id is reachable"
    );
    kani::cover!(id_bytes.len() == ID_BYTES * 2, "max-length id is reachable");
    kani::cover!(sequence == 0, "zero sequence is reachable");
    kani::cover!(sequence == u64::MAX, "max sequence is reachable");
}

#[kani::proof]
#[kani::unwind(48)]
fn check_observation_key_null_byte_id() {
    let id = "id\0with\0null";
    let sequence: u64 = kani::any();

    for table in Table::ALL {
        let key = observation_key(table, id, sequence);
        let parsed = split_observation_key(&key);
        assert_eq!(
            parsed.map(|(table, _, _)| table),
            Some(table.file().as_bytes())
        );
        assert_eq!(parsed.map(|(_, id, _)| id), Some(id.as_bytes()));
        assert_eq!(parsed.map(|(_, _, sequence)| sequence), Some(sequence));
    }
}
#[kani::proof]
#[kani::unwind(48)]
fn check_observation_key_zero_and_max_sequence() {
    let id = "test_id";

    for sequence in [0_u64, 1, 0x100, 0xffff_ffff, u64::MAX - 1, u64::MAX] {
        let key = observation_key(Table::Schools, id, sequence);
        let parsed = split_observation_key(&key);
        assert_eq!(
            parsed.map(|(table, _, _)| table),
            Some(Table::Schools.file().as_bytes())
        );
        assert_eq!(parsed.map(|(_, id, _)| id), Some(id.as_bytes()));
        assert_eq!(
            parsed.map(|(_, _, sequence)| sequence),
            Some(sequence),
            "sequence was misparsed"
        );
    }
}

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
        assert_eq!(
            sequence.to_be_bytes().as_slice(),
            &bytes[RAW_KEY_BYTES - 8..],
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
            assert!(
                matches!(parsed.as_deref(), Ok(value) if value == id.as_str()),
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
