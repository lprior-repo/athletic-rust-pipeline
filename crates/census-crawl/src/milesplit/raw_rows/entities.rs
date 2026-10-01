use std::borrow::Cow;

use super::super::parse::meet_index::html_unescape as decode_named_entities;

const MAX_ENTITY_DIGITS: usize = 7;

pub(super) fn html_unescape(value: &str) -> String {
    decode_named_entities(numeric_entities(value).as_ref())
}

fn numeric_entities(value: &str) -> Cow<'_, str> {
    if !value.contains("&#") {
        return Cow::Borrowed(value);
    }
    let mut decoded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find("&#") {
        decoded.push_str(rest.get(..index).unwrap_or_default());
        let tail = rest.get(index..).unwrap_or_default();
        match numeric_entity(tail) {
            Some((character, consumed)) => {
                decoded.push(character);
                rest = tail.get(consumed..).unwrap_or_default();
            }
            None => {
                decoded.push_str("&#");
                rest = tail.get(2..).unwrap_or_default();
            }
        }
    }
    decoded.push_str(rest);
    Cow::Owned(decoded)
}

fn numeric_entity(tail: &str) -> Option<(char, usize)> {
    let body = tail.strip_prefix("&#")?;
    let (digits, radix, marker): (&str, u32, usize) = match body.strip_prefix(['x', 'X']) {
        Some(hex) => (hex, 16, 3),
        None => (body, 10, 2),
    };
    let end = digits
        .find(|character: char| !character.is_digit(radix))
        .unwrap_or(digits.len());
    let terminated = end > 0
        && end <= MAX_ENTITY_DIGITS
        && digits.get(end..).is_some_and(|rest| rest.starts_with(';'));
    if !terminated {
        return None;
    }
    let code = u32::from_str_radix(digits.get(..end)?, radix).ok()?;
    let character = char::from_u32(code)?;
    Some((character, marker.saturating_add(end).saturating_add(1)))
}
