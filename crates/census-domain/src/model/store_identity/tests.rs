use crate::model::{StoreIdentity, StoreIdentityError};
use std::collections::{BTreeSet, HashSet};
use std::fmt::Debug;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const LOWER: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const UPPER: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
const MIXED: &str = "0123456789aBcDeF0123456789aBcDeF0123456789aBcDeF0123456789aBcDeF";
const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

fn invalid(value: &str) -> TestResult {
    match StoreIdentity::parse(value) {
        Err(StoreIdentityError::InvalidFormat) => {}
        observed => return Err(format!("parse({value:?}) returned {observed:?}").into()),
    }
    match StoreIdentity::try_from(value.to_owned()) {
        Err(StoreIdentityError::InvalidFormat) => {}
        observed => return Err(format!("TryFrom({value:?}) returned {observed:?}").into()),
    }
    let encoded = serde_json::to_string(value)?;
    if serde_json::from_str::<StoreIdentity>(&encoded).is_ok() {
        return Err(format!("checked deserialization accepted {encoded}").into());
    }
    Ok(())
}

#[test]
fn valid_spellings_and_zero_identity_roundtrip_as_exact_json_strings() -> TestResult {
    for value in [LOWER, UPPER, MIXED, ZERO] {
        let parsed = StoreIdentity::parse(value)?;
        equal(parsed.as_str(), value)?;
        equal(StoreIdentity::try_from(value.to_owned())?, parsed.clone())?;
        let encoded = serde_json::to_string(&parsed)?;
        equal(encoded.as_str(), format!("\"{value}\"").as_str())?;
        equal(
            serde_json::from_str::<StoreIdentity>(&encoded)?,
            parsed.clone(),
        )?;
        equal(serde_json::to_value(&parsed)?, serde_json::json!(value))?;
        equal(String::from(parsed), value.to_owned())?;
    }
    Ok(())
}

#[test]
fn equality_hash_and_order_use_retained_bytes_including_case() -> TestResult {
    let mut hashed = HashSet::new();
    let mut ordered = BTreeSet::new();
    for left in [LOWER, UPPER, MIXED, ZERO] {
        let identity = StoreIdentity::parse(left)?;
        equal(hashed.insert(identity.clone()), true)?;
        equal(ordered.insert(identity.clone()), true)?;
        for right in [LOWER, UPPER, MIXED, ZERO] {
            let other = StoreIdentity::parse(right)?;
            equal(identity == other, left == right)?;
            equal(identity.cmp(&other), left.cmp(right))?;
        }
    }
    equal(hashed.len(), 4)?;
    equal(ordered.len(), 4)?;
    let retained: Vec<_> = ordered.into_iter().map(String::from).collect();
    equal(
        retained,
        vec![
            ZERO.to_owned(),
            UPPER.to_owned(),
            MIXED.to_owned(),
            LOWER.to_owned(),
        ],
    )
}

#[test]
fn invalid_lengths_whitespace_prefixes_separators_nonhex_and_unicode_are_refused() -> TestResult {
    let values = [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        format!(" {LOWER}"),
        format!("{LOWER} "),
        format!("\t{LOWER}"),
        format!("{LOWER}\n"),
        format!("0x{LOWER}"),
        format!("{} ", "a".repeat(63)),
        format!("{}-", "a".repeat(63)),
        format!("{}g", "a".repeat(63)),
        format!("{}G", "a".repeat(63)),
        "é".repeat(32),
        format!("{}é", "a".repeat(62)),
        "Ａ".repeat(64),
    ];
    for value in values {
        invalid(&value)?;
    }
    Ok(())
}

#[test]
fn every_ascii_byte_in_each_identity_position_matches_the_single_invariant() -> TestResult {
    for position in 0..64_usize {
        let tail = 63_usize
            .checked_sub(position)
            .ok_or("position exceeds the identity bound")?;
        for byte in 0..=127_u8 {
            let value = format!(
                "{}{}{}",
                "a".repeat(position),
                char::from(byte),
                "a".repeat(tail)
            );
            if byte.is_ascii_hexdigit() {
                equal(StoreIdentity::parse(&value)?.as_str(), value.as_str())?;
            } else {
                invalid(&value)?;
            }
        }
    }
    Ok(())
}

#[test]
fn serde_rejects_nonstring_roots_and_malformed_json() -> TestResult {
    let forms = [
        "null".to_owned(),
        "true".to_owned(),
        "42".to_owned(),
        "0.5".to_owned(),
        "[]".to_owned(),
        format!("[\"{LOWER}\"]"),
        "{}".to_owned(),
        format!("{{\"value\":\"{LOWER}\"}}"),
        format!("{{\"StoreIdentity\":\"{LOWER}\"}}"),
        format!("\"{LOWER}"),
        format!("\"{LOWER}\" trailing"),
    ];
    for raw in forms {
        if serde_json::from_str::<StoreIdentity>(&raw).is_ok() {
            return Err(
                format!("serde accepted a nonstring or malformed representation: {raw}").into(),
            );
        }
    }
    Ok(())
}
