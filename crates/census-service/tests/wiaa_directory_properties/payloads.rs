use super::seam_config;
use census_crawl::wiaa::decode_cfemail;
use proptest::prelude::*;

const HEX: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];
const LOCAL: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '_',
    '-',
];
const DOMAIN: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-',
];

fn encode(address: &str, key: u8) -> String {
    let mut bytes = vec![key];
    bytes.extend(address.bytes().map(|byte| byte ^ key));
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

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
