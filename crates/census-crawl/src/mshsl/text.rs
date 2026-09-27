use regex::Regex;
use std::sync::LazyLock;

use super::MAX_CFEMAIL_HEX;

static CF_HREF: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"email-protection#([0-9a-fA-F]+)"#).ok());
static CF_ATTR: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"data-cfemail="([0-9a-fA-F]+)""#).ok());

pub fn decode_cfemail(encoded: &str) -> Option<String> {
    let hex = encoded.trim();
    if hex.is_empty() || hex.len() > MAX_CFEMAIL_HEX || !hex.len().is_multiple_of(2) {
        return None;
    }
    if !hex.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    let digits: Vec<char> = hex.chars().collect();
    let mut bytes: Vec<u8> = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let high = pair.first()?.to_digit(16)?;
        let low = pair.get(1)?.to_digit(16)?;
        let nibble = high.checked_mul(16)?.checked_add(low)?;
        bytes.push(u8::try_from(nibble).ok()?);
    }
    let key = *bytes.first()?;
    let decoded: Vec<u8> = bytes.iter().skip(1).map(|byte| byte ^ key).collect();
    let address = String::from_utf8(decoded).ok()?.trim().to_string();
    (!address.is_empty()).then_some(address)
}

pub fn decode_cfemail_fragment(fragment: &str) -> Vec<String> {
    let mut addresses: Vec<String> = Vec::new();
    for pattern in [CF_HREF.as_ref(), CF_ATTR.as_ref()].into_iter().flatten() {
        for capture in pattern.captures_iter(fragment) {
            let Some(token) = capture.get(1) else {
                continue;
            };
            let Some(address) = decode_cfemail(token.as_str()) else {
                continue;
            };
            if !addresses.contains(&address) {
                addresses.push(address);
            }
        }
    }
    addresses
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub(super) fn clean(value: &str) -> String {
    decode_entities(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "sir" | "rev"
        ) {
            parts.remove(0);
        } else {
            break;
        }
    }
    if parts.is_empty() {
        clean(value)
    } else {
        parts.join(" ")
    }
}
