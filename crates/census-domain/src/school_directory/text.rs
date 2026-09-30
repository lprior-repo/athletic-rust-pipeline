use super::DirectoryError;

const LOWERCASE_IN_PROSE: [&str; 14] = [
    "of", "and", "the", "at", "on", "in", "for", "to", "by", "de", "del", "la", "van", "von",
];

pub(super) fn collapse_whitespace(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut separator = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            separator = !out.is_empty();
            continue;
        }
        if separator {
            out.push(' ');
            separator = false;
        }
        out.push(ch);
    }
    out
}

pub(super) fn reject_control(value: &str, field: &'static str) -> Result<(), DirectoryError> {
    if value
        .chars()
        .any(|ch| ch.is_control() && !ch.is_whitespace())
    {
        Err(DirectoryError::ControlCharacter { field })
    } else {
        Ok(())
    }
}

pub(super) fn bounded(
    raw: &str,
    field: &'static str,
    limit: usize,
    shape: fn(&str) -> String,
) -> Result<String, DirectoryError> {
    reject_control(raw, field)?;
    let value = shape(&collapse_whitespace(raw));
    if value.is_empty() {
        return Err(DirectoryError::EmptyField { field });
    }
    let length = value.chars().count();
    if length > limit {
        return Err(DirectoryError::FieldTooLong {
            field,
            limit,
            length,
        });
    }
    Ok(value)
}

pub(super) fn as_is(value: &str) -> String {
    value.to_string()
}

pub(super) fn presentation_case(value: &str) -> String {
    if value.chars().any(|ch| ch.is_ascii_lowercase()) {
        return value.to_string();
    }
    let words: Vec<&str> = value.split(' ').collect();
    let last = words.len().saturating_sub(1);
    let mut out = String::with_capacity(value.len());
    for (index, word) in words.iter().enumerate() {
        if !out.is_empty() {
            out.push(' ');
        }
        let lowered = word.to_lowercase();
        let interior = index != 0 && index != last;
        if interior && LOWERCASE_IN_PROSE.contains(&lowered.as_str()) {
            out.push_str(&lowered);
        } else {
            out.push_str(&title_word(word));
        }
    }
    out
}

fn title_word(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut boundary = true;
    for ch in word.chars() {
        if boundary {
            let mut uppercase = ch.to_uppercase();
            if let Some(first) = uppercase.next() {
                out.push(first);
            }
            out.extend(uppercase.flat_map(char::to_lowercase));
        } else {
            out.extend(ch.to_lowercase());
        }
        boundary = ch == '-';
    }
    out
}
