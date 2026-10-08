use super::{buffer, DirectoryError};

pub(super) fn collapse(value: &str) -> Result<String, DirectoryError> {
    let mut out = buffer(value.len())?;
    let mut separator = false;
    value
        .chars()
        .for_each(|ch| append_collapsed(&mut out, ch, &mut separator));
    out.truncate(out.trim_end().len());
    Ok(out)
}

fn append_collapsed(out: &mut String, ch: char, separator: &mut bool) {
    if ch == '\n' {
        if !out.is_empty() {
            out.push('\n');
        }
        *separator = false;
    } else if ch.is_whitespace() {
        *separator = !out.is_empty();
    } else {
        if *separator {
            out.push(' ');
        }
        *separator = false;
        out.push(ch);
    }
}

pub(super) fn decode_entities(value: &str) -> Result<String, DirectoryError> {
    let mut out = buffer(value.len())?;
    let mut consumed = 0;
    value.char_indices().for_each(|(index, ch)| {
        if index < consumed {
            return;
        }
        let entity = (ch == '&')
            .then(|| value.get(index..))
            .flatten()
            .and_then(entity);
        match entity {
            Some((decoded, bytes)) => {
                out.push(decoded);
                consumed = index.saturating_add(bytes);
            }
            None => out.push(ch),
        }
    });
    Ok(out)
}

fn entity(tail: &str) -> Option<(char, usize)> {
    let end = tail.find(';').filter(|end| {
        let position = *end;
        position <= 12
    })?;
    let name = tail.get(1..end)?;
    let decoded = match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => name
            .strip_prefix('#')
            .and_then(|digits| {
                let decoded = digits.parse::<u32>();
                decoded.ok()
            })
            .and_then(char::from_u32),
    }?;
    Some((decoded, end.saturating_add(1)))
}
