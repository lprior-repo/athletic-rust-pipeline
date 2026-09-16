use super::{MAX_EXCEL_COLUMN, MAX_EXCEL_ROW};
use anyhow::{bail, Context, Result};
use quick_xml::events::BytesStart;

#[derive(Debug, Default)]
pub(crate) struct CellState {
    pub(crate) reference: String,
    pub(crate) cell_type: String,
    pub(crate) value: String,
}

pub(crate) fn cell_state(event: &BytesStart<'_>) -> Result<CellState> {
    Ok(CellState {
        reference: option_string(attribute(event, b"r")?),
        cell_type: option_string(attribute(event, b"t")?),
        value: String::new(),
    })
}

pub(crate) fn validate_cell_row(reference: &str, expected: u32) -> Result<usize> {
    let digit_start = reference
        .bytes()
        .position(|byte| byte.is_ascii_digit())
        .with_context(|| format!("invalid cell reference {reference:?}"))?;
    let row = reference
        .get(digit_start..)
        .with_context(|| format!("invalid cell reference {reference:?}"))?
        .parse::<u32>()
        .with_context(|| format!("invalid cell reference {reference:?}"))?;
    if row == 0 || row > MAX_EXCEL_ROW || row != expected {
        bail!("cell reference {reference:?} does not match worksheet row {expected}");
    }
    column_index(reference)
}

pub(crate) fn column_index(reference: &str) -> Result<usize> {
    let mut result = 0_usize;
    let mut letters = 0_usize;
    let mut digits_started = false;
    reference.bytes().try_for_each(|byte| {
        if !digits_started && byte.is_ascii_alphabetic() {
            let offset = byte.to_ascii_uppercase().checked_sub(b'A');
            let offset = offset.and_then(|value| value.checked_add(1));
            let offset = offset.with_context(|| format!("invalid cell reference {reference:?}"))?;
            result = result
                .checked_mul(26)
                .and_then(|value| value.checked_add(usize::from(offset)))
                .with_context(|| format!("cell reference column overflow: {reference:?}"))?;
            letters = letters.saturating_add(1);
            Ok(())
        } else if byte.is_ascii_digit() {
            digits_started = true;
            Ok(())
        } else {
            bail!("invalid cell reference {reference:?}");
        }
    })?;
    if letters == 0 || !digits_started {
        bail!("invalid cell reference {reference:?}");
    }
    let column = result
        .checked_sub(1)
        .with_context(|| format!("invalid cell reference {reference:?}"))?;
    if column >= MAX_EXCEL_COLUMN {
        bail!("cell reference column exceeds Excel range: {reference:?}");
    }
    Ok(column)
}

pub(crate) fn attribute(event: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>> {
    event
        .attributes()
        .with_checks(false)
        .try_fold(None, |found, item| {
            let attribute = item?;
            if attribute.key.as_ref() == key {
                Ok(Some(decode_xml_text(attribute.value.as_ref())?))
            } else {
                Ok(found)
            }
        })
}

pub(crate) fn decode_xml_text(value: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(value)?;
    Ok(quick_xml::escape::unescape(text)?.into_owned())
}

pub(crate) fn decode_reference(reference: &quick_xml::events::BytesRef<'_>) -> Result<char> {
    if let Some(value) = reference.resolve_char_ref()? {
        if matches!(value, '\t' | '\n' | '\r')
            || ('\u{20}'..='\u{d7ff}').contains(&value)
            || ('\u{e000}'..='\u{fffd}').contains(&value)
            || value >= '\u{10000}'
        {
            return Ok(value);
        }
        bail!("XML reference contains a forbidden character");
    }
    let bytes: &[u8] = reference;
    match bytes {
        b"amp" => Ok('&'),
        b"lt" => Ok('<'),
        b"gt" => Ok('>'),
        b"quot" => Ok('"'),
        b"apos" => Ok('\''),
        _ => bail!("unsupported XML entity reference"),
    }
}

pub(crate) fn option_string(value: Option<String>) -> String {
    value.unwrap_or_default()
}

pub(crate) fn column_name(mut index: usize) -> String {
    let mut output = Vec::new();
    index = index.saturating_add(1);
    while index > 0 {
        let adjusted = match index.checked_sub(1) {
            Some(value) => value,
            None => return String::new(),
        };
        let remainder = adjusted % 26;
        let offset = match u8::try_from(remainder) {
            Ok(value) => value,
            Err(_) => return String::new(),
        };
        let byte = match b'A'.checked_add(offset) {
            Some(value) => value,
            None => return String::new(),
        };
        output.push(char::from(byte));
        index = adjusted / 26;
    }
    output.iter().rev().collect()
}

pub(crate) fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
