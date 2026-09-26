//! The markup primitives every reader in this tree shares.
//!
//! The crate parses HTML with each adapter's own primitives (see `ohsaa::parse`,
//! `wiaa::primitives`): there is no shared HTML tree, so these are deliberately minimal and
//! structural — one cell by its `data-label`, a fragment's links and attributes, and the text
//! runs a fragment publishes.

/// Every text run a fragment publishes, in document order: tags dropped, entities decoded, whitespace
/// collapsed, empty runs skipped.
pub(super) fn text_runs(fragment: &str) -> impl Iterator<Item = String> + '_ {
    fragment
        .split('<')
        .enumerate()
        .filter_map(|(index, piece)| {
            let text = if index == 0 {
                piece
            } else {
                piece.get(piece.find('>')?.checked_add(1)?..)?
            };
            let run = collapse_whitespace(&decode_entities(text));
            (!run.is_empty()).then_some(run)
        })
}

/// The content of the cell the row marks with `data-label="<label>"`, up to the next cell.
///
/// Cells are addressed by label rather than by position: the host renders the same row with and
/// without its extra `Conv`/`Wind` columns, and a positional reader would mis-assign the mark.
pub(super) fn cell<'a>(row: &'a str, label: &str) -> Option<&'a str> {
    let marker = format!("data-label=\"{label}\"");
    let start = row.find(&marker)?.checked_add(marker.len())?;
    let after = row.get(start..)?;
    let body = after.get(after.find('>')?.checked_add(1)?..)?;
    let end = body.find("<div").unwrap_or(body.len());
    body.get(..end)
}

/// Every `<a href=…>text</a>` in a fragment, as (href, display text).
pub(super) fn links(fragment: &str) -> Vec<(&str, String)> {
    let mut found = Vec::new();
    for piece in fragment.split("<a ").skip(1) {
        let Some(href) = attribute(piece, "href") else {
            continue;
        };
        let text = piece
            .find('>')
            .and_then(|open| piece.get(open.checked_add(1)?..))
            .and_then(|body| body.get(..body.find("</a>")?))
            .map(text_of)
            .unwrap_or_default();
        found.push((href, text));
    }
    found
}

/// The value of one attribute inside a tag fragment.
pub(super) fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}=\"");
    let start = tag.find(&marker)?.checked_add(marker.len())?;
    let after = tag.get(start..)?;
    after.get(..after.find('"')?)
}

/// A cell's or fragment's text: tags removed, entities decoded, whitespace collapsed.
pub(super) fn text_of(fragment: &str) -> String {
    collapse_whitespace(&decode_entities(&strip_tags(fragment)))
}

/// Remove HTML tags, keeping one space where a tag separated two words.
fn strip_tags(fragment: &str) -> String {
    let mut result = String::with_capacity(fragment.len());
    let mut in_tag = false;
    let mut bytes = fragment.as_bytes().iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        match byte {
            b'<' => in_tag = true,
            b'>' => {
                in_tag = false;
                if bytes.peek().is_some_and(|next| *next != b' ') {
                    result.push(' ');
                }
            }
            other if !in_tag => result.push(char::from(other)),
            _ => {}
        }
    }
    collapse_whitespace(&result)
}

/// Decode the entities the host escapes its cell text with (`&amp;`, `&quot;`, `&#39;`, `&nbsp;`).
pub(super) fn decode_entities(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '&' {
            result.push(ch);
            continue;
        }
        let mut entity = String::new();
        let mut terminated = false;
        for next in chars.by_ref() {
            if next == ';' {
                terminated = true;
                break;
            }
            entity.push(next);
        }
        match entity.as_str() {
            "amp" => result.push('&'),
            "lt" => result.push('<'),
            "gt" => result.push('>'),
            "quot" => result.push('"'),
            "apos" | "#39" => result.push('\''),
            "nbsp" => result.push(' '),
            _ => {
                if terminated {
                    if let Some(decoded) = numeric_entity(&entity) {
                        result.push(decoded);
                    } else {
                        result.push('&');
                        result.push_str(&entity);
                        result.push(';');
                    }
                } else {
                    result.push('&');
                    result.push_str(&entity);
                }
            }
        }
    }
    result
}

/// The character a numeric entity names (`#39`, `#x27`).
fn numeric_entity(entity: &str) -> Option<char> {
    let digits = entity.strip_prefix('#')?;
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };
    char::from_u32(code)
}

/// Collapse every run of whitespace to one space and trim the ends.
pub(super) fn collapse_whitespace(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut previous_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            if !previous_space {
                result.push(' ');
                previous_space = true;
            }
        } else {
            previous_space = false;
            result.push(ch);
        }
    }
    result.trim().to_string()
}
