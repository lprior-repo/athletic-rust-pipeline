//! The addresses MSHSL hides, in both the forms its pages use.
//!
//! The server renders `mailto:?email-protection#<hex>` hrefs and a DOM snapshot can carry the same
//! payload as a `data-cfemail` attribute; either way the payload is the Cloudflare scheme — the key
//! byte, then every address byte XORed with it, as hex — and `decode_cfemail_fragment` reads both
//! forms out of one fragment. The laws: the decoder is the inverse of the encoding the provider
//! applies (so a crawl cannot silently drop a coach's address), each hidden address is published
//! once even when a page hides it twice, and the two forms hide the same addresses.

use super::seam_config;
use midwest_census::sources::mshsl::{decode_cfemail, decode_cfemail_fragment};
use proptest::prelude::*;

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

/// An address the league publishes: a lower-case local part and a dotted lower-case domain, with no
/// blanks and no leading or trailing space, so the decoder returns exactly it.
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

/// The addresses one page hides, in the order it hides them — every third address is hidden twice,
/// which is what a coach holding two roles looks like on one page.
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

/// The addresses the way a page hides them, one `email-protection#` href per address.
fn render_hrefs(addresses: &[String]) -> String {
    let mut body = String::from("<div class=\"grid__item\">\n");
    for (index, address) in addresses.iter().enumerate() {
        let key = 0x40u8.wrapping_add(u8::try_from(index).unwrap_or(0).wrapping_mul(7));
        body.push_str(&format!(
            "<a href=\"mailto:?email-protection#{}\" class=\"fieldValue\">{}</a>\n",
            encode(address, key),
            address
        ));
    }
    body.push_str("</div>\n");
    body
}

/// The same addresses the way a DOM snapshot hides them: as `data-cfemail` attributes.
fn render_attrs(addresses: &[String]) -> String {
    let mut body = String::from("<div class=\"grid__item\">\n");
    for (index, address) in addresses.iter().enumerate() {
        let key = 0x40u8.wrapping_add(u8::try_from(index).unwrap_or(0).wrapping_mul(7));
        body.push_str(&format!(
            "<span data-cfemail=\"{}\"></span>\n",
            encode(address, key)
        ));
    }
    body.push_str("</div>\n");
    body
}

/// Each address once, in the order it first appears.
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

    /// The decoder is the inverse of the encoding the page's own script applies, and the blanks
    /// around a payload belong to the attribute rather than to the address.
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

    /// A page that hides an address twice publishes it once, in the order it is first hidden.
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

    /// The href form and the `data-cfemail` form hide the same addresses: whichever way a capture was
    /// taken, the crawl reads the same contacts.
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
