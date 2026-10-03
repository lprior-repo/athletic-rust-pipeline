use std::collections::BTreeMap;

const PREFIX: [char; 7] = ['c', 'r', 'a', 't', 'e', ':', ':'];

pub(super) fn refs_in_line(line: &str) -> Vec<String> {
    if line.trim_start().starts_with("//") {
        return Vec::new();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut targets = Vec::new();
    let mut cursor = 0usize;
    while let Some(position) = find_from(&chars, &PREFIX, cursor) {
        cursor = position.saturating_add(PREFIX.len());
        if inside_string(&chars, position) {
            continue;
        }
        let rest = chars
            .get(cursor..)
            .map_or(Default::default(), core::convert::identity);
        if rest.first() == Some(&'{') {
            group_targets(rest, &mut targets);
        } else if let Some(name) = leading_ident(rest) {
            push_target(name, &mut targets);
        }
    }
    targets
}

pub(super) fn crates_in_line(line: &str, aliases: &BTreeMap<String, String>) -> Vec<String> {
    if line.trim_start().starts_with("//") {
        return Vec::new();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut targets = Vec::new();
    for (alias, package) in aliases {
        let needle: Vec<char> = alias.chars().collect();
        let mut cursor = 0usize;
        while let Some(position) = find_from(&chars, &needle, cursor) {
            cursor = position.saturating_add(needle.len());
            let starts = position == 0
                || chars
                    .get(position.saturating_sub(1))
                    .is_some_and(|ch| !is_ident_char(*ch));
            let roots = chars.get(cursor) == Some(&':')
                && chars.get(cursor.saturating_add(1)) == Some(&':');
            if starts && roots && !inside_string(&chars, position) {
                targets.push(package.clone());
            }
        }
    }
    targets
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn group_targets(chars: &[char], targets: &mut Vec<String>) {
    let mut depth = 0usize;
    let mut item: Vec<char> = Vec::new();
    for ch in chars {
        match ch {
            '{' => {
                depth = depth.saturating_add(1);
                if depth > 1 {
                    item.push(*ch);
                }
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    push_item(&item, targets);
                    return;
                }
                item.push(*ch);
            }
            ',' if depth == 1 => {
                push_item(&item, targets);
                item.clear();
            }
            _ => item.push(*ch),
        }
    }
}

fn push_item(item: &[char], targets: &mut Vec<String>) {
    if let Some(name) = leading_ident(item) {
        push_target(name, targets);
    }
}

fn push_target(name: String, targets: &mut Vec<String>) {
    if name != "self" && name != "super" && name != "crate" {
        targets.push(name);
    }
}

fn leading_ident(chars: &[char]) -> Option<String> {
    let mut rest = chars;
    while rest.first().is_some_and(|ch| ch.is_whitespace()) {
        rest = rest.get(1..)?;
    }
    if rest.first() == Some(&'r') && rest.get(1) == Some(&'#') {
        rest = rest.get(2..)?;
    }
    let mut name = String::new();
    for ch in rest {
        if ch.is_ascii_alphanumeric() || *ch == '_' {
            name.push(*ch);
        } else {
            break;
        }
    }
    match name.chars().next() {
        Some(first) if !first.is_ascii_digit() => Some(name),
        _ => None,
    }
}

fn inside_string(chars: &[char], at: usize) -> bool {
    let mut open = false;
    let mut previous = ' ';
    for ch in chars.iter().take(at) {
        if *ch == '"' && previous != '\\' {
            open = !open;
        }
        previous = *ch;
    }
    open
}

fn find_from(hay: &[char], needle: &[char], start: usize) -> Option<usize> {
    let width = needle.len();
    let last = hay.len().checked_sub(width)?;
    (start..=last).find(|index| hay.get(*index..index.saturating_add(width)) == Some(needle))
}
