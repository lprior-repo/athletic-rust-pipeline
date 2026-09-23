//! Resolving `crate::…` paths in one line to the top-level modules they name.
//!
//! The parse is deliberately lexical: this is a ratchet over a module graph, not a compiler, and the
//! only failure mode that matters is *hiding* an edge. Every approximation here over-reports instead —
//! a `crate::{a::A, b::B}` group contributes both items, a nested group contributes its outer
//! segment, and a path that a stricter parse would ignore still counts as a reference because a
//! false positive is caught in review while a missed seam is not caught at all. The one exception is
//! a reference inside a string literal, skipped by quote parity, because the module resolver in a
//! fixture path is not a dependency of anything.

use std::collections::BTreeMap;

/// `crate::` — the only prefix that points into this crate's own module tree.
const PREFIX: [char; 7] = ['c', 'r', 'a', 't', 'e', ':', ':'];

/// Every top-level module a line references through `crate::…`.
///
/// A `crate::{a::A, b::B}` group contributes each item's first segment; a plain path contributes
/// its first segment. References that sit inside a string literal are skipped with a quote-parity
/// approximation — good enough for a ratchet, and it can only over-report, never hide.
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
        let rest = chars.get(cursor..).unwrap_or_default();
        if rest.first() == Some(&'{') {
            group_targets(rest, &mut targets);
        } else if let Some(name) = leading_ident(rest) {
            push_target(name, &mut targets);
        }
    }
    targets
}

/// Every workspace crate the line names through one of its aliases (`census_store::Table`).
///
/// The same lexical bargain as [`refs_in_line`], and one more approximation in the same direction:
/// the answer is the *package* the alias belongs to, because the crate table is about crates and not
/// about the identifiers a manifest happens to give them. Matching is anchored at both ends of the
/// identifier, so `census_store_x` and `my_census_store` name nothing, and the identifier has to be
/// followed by `::` — the path root — so a field whose name repeats a crate does not either.
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

/// Whether the character may sit inside an identifier.
fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// The module names a brace group contributes: each item's first segment.
///
/// `{store::Store, report::Error}` contributes `store` and `report`; nesting contributes the outer
/// first segment (`{store::{Store, Table}}` contributes `store`), which is what a top-level table
/// needs.
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

/// One group item's contribution, dropped when it names the group rather than a module.
fn push_item(item: &[char], targets: &mut Vec<String>) {
    if let Some(name) = leading_ident(item) {
        push_target(name, targets);
    }
}

/// Record a target unless it is a path keyword rather than a module.
fn push_target(name: String, targets: &mut Vec<String>) {
    if name != "self" && name != "super" && name != "crate" {
        targets.push(name);
    }
}

/// The leading identifier of a path fragment, raw-string prefix stripped, or `None`.
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

/// Whether the character at `at` sits inside a simple string literal.
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

/// The first position at or after `start` where `needle` occurs.
fn find_from(hay: &[char], needle: &[char], start: usize) -> Option<usize> {
    let width = needle.len();
    let last = hay.len().checked_sub(width)?;
    (start..=last).find(|index| hay.get(*index..index.saturating_add(width)) == Some(needle))
}
