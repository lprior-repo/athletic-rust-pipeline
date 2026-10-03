use super::lexer::{after, is_ident_continue, is_ident_start, take_while, Kind};

pub(super) fn block_comment(rest: &str) -> (Kind, &str) {
    let bytes = rest.as_bytes();
    let mut index = 2usize;
    let mut depth = 1u32;
    while let Some(current) = bytes.get(index).copied() {
        match current {
            b'/' if bytes.get(index.saturating_add(1)).copied() == Some(b'*') => {
                depth = depth.saturating_add(1);
                index = index.saturating_add(2);
            }
            b'*' if bytes.get(index.saturating_add(1)).copied() == Some(b'/') => {
                depth = depth.saturating_sub(1);
                index = index.saturating_add(2);
                if depth == 0 {
                    return (Kind::BlockComment { terminated: true }, after(rest, index));
                }
            }
            _ => index = index.saturating_add(1),
        }
    }
    (Kind::BlockComment { terminated: false }, "")
}

pub(super) fn string(rest: &str, prefix: usize) -> (Kind, &str) {
    let body = after(rest, prefix.saturating_add(1));
    match unescaped_quote(body, '"') {
        Some(index) => (literal(true), after(body, index.saturating_add(1))),
        None => (literal(false), ""),
    }
}

pub(super) fn raw_string(rest: &str, before_r: usize, hashes: usize) -> (Kind, &str) {
    let body = after(rest, before_r.saturating_add(1).saturating_add(hashes));
    let bytes = body.as_bytes();
    let mut index = 1usize;
    while let Some(current) = bytes.get(index).copied() {
        if current == b'"' && closes_raw(bytes, index.saturating_add(1), hashes) {
            let end = index.saturating_add(1).saturating_add(hashes);
            return (literal(true), after(body, end));
        }
        index = index.saturating_add(1);
    }
    (literal(false), "")
}

pub(super) fn ident(rest: &str) -> (Kind, &str) {
    let end = take_while(rest, 0, is_ident_continue);
    let name = rest.get(..end).map_or(Default::default(), core::convert::identity);
    let tail = after(rest, end);
    match name {
        "b" | "c" if tail.starts_with('"') => string(rest, end),
        "b" if tail.starts_with('\'') => char_literal(rest, end),
        "r" | "br" | "cr" => raw_prefix(rest, end),
        _ => (Kind::Ident, tail),
    }
}

pub(super) fn quote(rest: &str) -> (Kind, &str) {
    let body = after(rest, 1);
    let first = body.chars().next();
    let second = body.chars().nth(1);
    let lifetime = match first {
        None => false,
        Some(_) if second == Some('\'') => false,
        Some(character) => is_ident_start(character) || character.is_ascii_digit(),
    };
    if lifetime {
        lifetime_or_char(rest)
    } else {
        char_literal(rest, 0)
    }
}

fn raw_prefix(rest: &str, prefix: usize) -> (Kind, &str) {
    let tail = after(rest, prefix);
    let hashes = tail
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let body = after(tail, hashes);
    if body.starts_with('"') {
        return raw_string(rest, prefix.saturating_sub(1), hashes);
    }
    if prefix == 1 && hashes == 1 && body.chars().next().is_some_and(is_ident_start) {
        let end = take_while(rest, prefix.saturating_add(1), is_ident_continue);
        return (Kind::RawIdent, after(rest, end));
    }
    if hashes == 0 {
        return (Kind::Ident, tail);
    }
    raw_string(rest, prefix.saturating_sub(1), hashes)
}

fn lifetime_or_char(rest: &str) -> (Kind, &str) {
    let first = after(rest, 1).chars().next();
    let starts_with_number = first.is_some_and(|character| character.is_ascii_digit());
    let content = match first {
        Some(character) => 1usize.saturating_add(character.len_utf8()),
        None => 1usize,
    };
    let end = take_while(rest, content, is_ident_continue);
    let tail = after(rest, end);
    if tail.starts_with('\'') {
        return (literal(true), after(tail, 1));
    }
    if tail.starts_with('#') && !starts_with_number {
        let raw = take_while(rest, end.saturating_add(1), is_ident_continue);
        return (Kind::Other, after(rest, raw));
    }
    (Kind::Other, tail)
}

fn char_literal(rest: &str, prefix: usize) -> (Kind, &str) {
    let body = after(rest, prefix.saturating_add(1));
    let first = body.chars().next();
    let second = body.chars().nth(1);
    if second == Some('\'') && first != Some('\\') {
        let consumed = first.map_or(0, char::len_utf8);
        return (literal(true), after(body, consumed.saturating_add(1)));
    }
    let mut chars = body.char_indices();
    while let Some((index, character)) = chars.next() {
        match character {
            '\'' => return (literal(true), after(body, index.saturating_add(1))),
            '/' => return (literal(false), after(body, index)),
            '\n' if chars.clone().next().map(|(_, next)| next) != Some('\'') => {
                return (literal(false), after(body, index));
            }
            '\\' => {
                chars.next();
            }
            _ => {}
        }
    }
    (literal(false), "")
}

fn unescaped_quote(body: &str, quote: char) -> Option<usize> {
    let mut chars = body.char_indices();
    while let Some((index, character)) = chars.next() {
        if character == '\\' {
            chars.next();
            continue;
        }
        if character == quote {
            return Some(index);
        }
    }
    None
}

fn closes_raw(bytes: &[u8], start: usize, hashes: usize) -> bool {
    (0..hashes).all(|offset| bytes.get(start.saturating_add(offset)).copied() == Some(b'#'))
}

fn literal(terminated: bool) -> Kind {
    Kind::Literal { terminated }
}
