use regex::Regex;
use std::sync::LazyLock;

use super::{MAX_CFEMAIL_HEX, MAX_LISTING_BYTES};
use crate::directory::acquisition::resource;
use crate::CrawlResult;

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

const ENTITIES: [(&str, char); 10] = [
    ("&#039;", '\''),
    ("&#39;", '\''),
    ("&#x27;", '\''),
    ("&apos;", '\''),
    ("&quot;", '"'),
    ("&nbsp;", ' '),
    ("&#160;", ' '),
    ("&lt;", '<'),
    ("&gt;", '>'),
    ("&amp;", '&'),
];

fn decoded(value: &str) -> impl Iterator<Item = char> + '_ {
    let mut remaining = value;
    std::iter::from_fn(move || {
        for (entity, character) in ENTITIES {
            if let Some(rest) = remaining.strip_prefix(entity) {
                remaining = rest;
                return Some(character);
            }
        }
        let character = remaining.chars().next()?;
        remaining = remaining.strip_prefix(character)?;
        Some(character)
    })
}

fn normalized(value: &str) -> impl Iterator<Item = char> + '_ {
    let mut started = false;
    let mut whitespace = false;
    decoded(value)
        .flat_map(move |character| {
            if character.is_whitespace() {
                whitespace = started;
                [None, None]
            } else {
                let space = whitespace.then_some(' ');
                started = true;
                whitespace = false;
                [space, Some(character)]
            }
        })
        .flatten()
}

fn listing_buffer(bytes: usize) -> CrawlResult<String> {
    if bytes > MAX_LISTING_BYTES {
        return Err(resource(
            "MSHSL listing text bytes",
            bytes,
            MAX_LISTING_BYTES,
        ));
    }
    let mut value = String::new();
    value
        .try_reserve_exact(bytes)
        .map_err(|_| resource("MSHSL listing text allocation", bytes, MAX_LISTING_BYTES))?;
    Ok(value)
}

pub(super) fn copy_bounded(value: &str) -> CrawlResult<String> {
    let mut copied = listing_buffer(value.len())?;
    copied.push_str(value);
    Ok(copied)
}

pub(super) fn clean_bounded(value: &str) -> CrawlResult<String> {
    let mut cleaned = listing_buffer(value.len())?;
    cleaned.extend(normalized(value));
    Ok(cleaned)
}

pub(super) fn clean(value: &str) -> String {
    normalized(value).collect()
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
