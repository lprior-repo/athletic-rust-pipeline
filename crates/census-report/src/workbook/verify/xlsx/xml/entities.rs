use crate::report::ReportResult;
use crate::workbook::verify::xlsx::Scope;

pub(super) fn decode(bytes: &[u8], scope: &Scope<'_>) -> ReportResult<String> {
    let invalid = |_| scope.corrupt("an XML text run is not valid UTF-8");
    if !bytes.contains(&b'&') {
        return std::str::from_utf8(bytes)
            .map(str::to_string)
            .map_err(invalid);
    }
    let mut text = String::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some(position) = rest.iter().position(|byte| *byte == b'&') {
        let (head, tail) = rest.split_at(position);
        text.push_str(std::str::from_utf8(head).map_err(invalid)?);
        let tail = tail
            .get(1..)
            .ok_or_else(|| scope.corrupt("an XML entity has no name"))?;
        let end = tail
            .iter()
            .position(|byte| *byte == b';')
            .ok_or_else(|| scope.corrupt("an XML entity has no terminator"))?;
        let (entity, after) = tail.split_at(end);
        let value =
            entity_char(entity).ok_or_else(|| scope.corrupt("an XML entity is not recognised"))?;
        text.push(value);
        rest = after.get(1..).map_or(&[], |tail| tail);
    }
    text.push_str(std::str::from_utf8(rest).map_err(invalid)?);
    Ok(text)
}

fn entity_char(entity: &[u8]) -> Option<char> {
    match entity {
        b"amp" => Some('&'),
        b"lt" => Some('<'),
        b"gt" => Some('>'),
        b"quot" => Some('"'),
        b"apos" => Some('\''),
        _ => numeric(entity),
    }
}

fn numeric(entity: &[u8]) -> Option<char> {
    let (radix, digits) = match entity.split_first() {
        Some((b'#', rest)) => match rest.split_first() {
            Some((b'x' | b'X', digits)) => (16, digits),
            _ => (10, rest),
        },
        _ => return None,
    };
    let value = u32::from_str_radix(std::str::from_utf8(digits).ok()?, radix).ok()?;
    if value == 0 {
        return None;
    }
    char::from_u32(value)
}
