//! The `data-cfemail` payloads the directory publishes, and what the decoder is allowed to answer.
//!
//! `decode_cfemail` is the seam's only reader of obfuscated data: the attribute carries the key byte
//! followed by every address byte XORed with it, written as hex, and the provider's
//! `email-decode.min.js` reverses exactly that. The laws here are the two that matter for a contact
//! import: the decoder is the inverse of the encoding the provider itself applies (so a refresh
//! cannot silently drop an address), and whatever junk an attribute carries, the decoder answers
//! with an address that could have been printed on the page — never a half-decoded string.

use super::seam_config;
use census_crawl::wiaa::decode_cfemail;
use proptest::prelude::*;

/// Lowers of the hex alphabet the provider writes payloads in.
const HEX: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];
/// Characters a published local part carries.
const LOCAL: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '_',
    '-',
];
/// Characters a published domain carries; the dot is placed by the generator.
const DOMAIN: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-',
];

/// The provider's own encoding of one address: the key byte, then every address byte XORed with it,
/// as lower-case hex.
fn encode(address: &str, key: u8) -> String {
    let mut bytes = vec![key];
    bytes.extend(address.bytes().map(|byte| byte ^ key));
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// An address the directory prints: a lower-case local part and a dotted lower-case domain, with no
/// blanks and no brackets, so `valid_email` accepts exactly it.
fn address() -> impl Strategy<Value = String> {
    let local = prop::collection::vec(prop::sample::select(LOCAL), 1..9);
    let label = prop::collection::vec(prop::sample::select(DOMAIN), 1..10);
    let tld = prop::collection::vec(prop::sample::select(DOMAIN), 2..4);
    (local, label, tld).prop_map(|(local, label, tld)| {
        format!(
            "{}@{}.{}",
            local.iter().collect::<String>(),
            label.iter().collect::<String>(),
            tld.iter().collect::<String>()
        )
    })
}

/// A `data-cfemail` attribute value: the length a real one carries, the lengths a truncated one
/// carries, the blanks an attribute picks up, and bytes outside the alphabet.
fn payload() -> impl Strategy<Value = String> {
    fn hex(max: usize) -> impl Strategy<Value = String> {
        prop::collection::vec(prop::sample::select(HEX), 0..max)
            .prop_map(|chars| chars.into_iter().collect::<String>())
    }
    prop_oneof![
        hex(40),
        hex(40).prop_map(|text| format!(" \t{text}\n")),
        prop::collection::vec(any::<char>(), 0..24)
            .prop_map(|chars| chars.into_iter().collect::<String>()),
    ]
}

proptest! {
    #![proptest_config(seam_config())]

    /// The decoder is the inverse of the provider's encoder, and the blanks around a payload belong
    /// to the attribute rather than to the address.
    #[test]
    fn the_providers_own_encoding_round_trips(address in address(), key in any::<u8>()) {
        let payload = encode(&address, key);
        let decoded = decode_cfemail(&payload);
        prop_assert_eq!(
            decoded.as_deref(),
            Some(address.as_str()),
            "the payload `{}` decodes back to `{}`",
            payload,
            address
        );
        let padded = decode_cfemail(&format!(" \t{payload}\n"));
        prop_assert_eq!(
            padded.as_deref(),
            decoded.as_deref(),
            "the attribute's blanks do not reach the address"
        );
    }

    /// Whatever the attribute holds, the decoder answers with a published address or with nothing: a
    /// half-decoded string can never become a `professional_email`.
    #[test]
    fn a_payload_decodes_to_a_published_address_or_to_nothing(payload in payload()) {
        let Some(address) = decode_cfemail(&payload) else {
            return Ok(());
        };
        prop_assert_eq!(
            address.as_str(),
            address.trim(),
            "an address is trimmed: {:?}",
            address
        );
        prop_assert!(
            !address.contains(char::is_whitespace),
            "an address carries no blanks: {:?}",
            address
        );
        prop_assert!(
            !address.contains(['[', ']']),
            "no bracketed address: {:?}",
            address
        );
        prop_assert_eq!(
            address.matches('@').count(),
            1,
            "one `@` per address: {:?}",
            address
        );
        let (local, domain) = address
            .split_once('@')
            .ok_or_else(|| TestCaseError::fail(format!("{address:?} has no `@`")))?;
        prop_assert!(!local.is_empty(), "a non-empty local part: {:?}", address);
        prop_assert!(domain.contains('.'), "a dotted domain: {:?}", address);
    }
}
