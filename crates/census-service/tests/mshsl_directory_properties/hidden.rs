use super::seam_config;
use census_crawl::mshsl::{decode_cfemail, decode_cfemail_fragment};
use proptest::prelude::*;

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

fn hidden_addresses() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(address(), 1..5).prop_map(|addresses| {
        addresses
            .into_iter()
            .enumerate()
            .flat_map(|(index, address)| {
                if index % 3 == 1 {
                    vec![address.clone(), address]
                } else {
                    vec![address]
                }
            })
            .collect()
    })
}

fn render_hrefs(addresses: &[String]) -> String {
    let mut body = String::from("<div class=\"grid__item\">\n");
    for (index, address) in addresses.iter().enumerate() {
        let key = 0x40u8.wrapping_add(u8::try_from(index).map_or(0, |value| value).wrapping_mul(7));
        body.push_str(&format!(
            "<a href=\"mailto:?email-protection#{}\" class=\"fieldValue\">{}</a>\n",
            encode(address, key),
            address
        ));
    }
    body.push_str("</div>\n");
    body
}

fn render_attrs(addresses: &[String]) -> String {
    let mut body = String::from("<div class=\"grid__item\">\n");
    for (index, address) in addresses.iter().enumerate() {
        let key = 0x40u8.wrapping_add(u8::try_from(index).map_or(0, |value| value).wrapping_mul(7));
        body.push_str(&format!(
            "<span data-cfemail=\"{}\"></span>\n",
            encode(address, key)
        ));
    }
    body.push_str("</div>\n");
    body
}

fn unique(addresses: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for address in addresses {
        if !out.contains(address) {
            out.push(address.clone());
        }
    }
    out
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
            Some(address.as_str()),
            "the attribute's blanks do not reach the address"
        );
    }

    #[test]
    fn a_fragment_publishes_each_hidden_address_once(addresses in hidden_addresses()) {
        let published = decode_cfemail_fragment(&render_hrefs(&addresses));
        prop_assert!(!published.is_empty(), "a page that hides an address publishes one");
        prop_assert_eq!(
            published,
            unique(&addresses),
            "each hidden address once, in the order the page hides them"
        );
    }

    #[test]
    fn the_two_forms_hide_the_same_addresses(addresses in hidden_addresses()) {
        let mut from_hrefs = decode_cfemail_fragment(&render_hrefs(&addresses));
        let mut from_attrs = decode_cfemail_fragment(&render_attrs(&addresses));
        from_hrefs.sort();
        from_attrs.sort();
        prop_assert_eq!(
            from_hrefs,
            from_attrs,
            "both forms of one page hide the same addresses"
        );
    }
}
