
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

pub(super) fn cell<'a>(row: &'a str, label: &str) -> Option<&'a str> {
    let marker = format!("data-label=\"{label}\"");
    let start = row.find(&marker)?.checked_add(marker.len())?;
    let after = row.get(start..)?;
    let body = after.get(after.find('>')?.checked_add(1)?..)?;
    let end = body.find("<div").unwrap_or(body.len());
    body.get(..end)
}

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

pub(super) fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}=\"");
    let start = tag.find(&marker)?.checked_add(marker.len())?;
    let after = tag.get(start..)?;
    after.get(..after.find('"')?)
}

pub(super) fn text_of(fragment: &str) -> String {
    collapse_whitespace(&decode_entities(&strip_tags(fragment)))
}

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

fn numeric_entity(entity: &str) -> Option<char> {
    let digits = entity.strip_prefix('#')?;
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse::<u32>().ok()?,
    };
    char::from_u32(code)
}

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
