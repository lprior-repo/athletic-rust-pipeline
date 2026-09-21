//! Text bound enforcement for the streaming capture: row text and display names
//! grow only through these checked appends.

use super::super::{normalize, MAX_TEXT_BYTES};

pub(super) fn append_raw(target: &mut String, text: &str, issue: &mut Option<String>) {
    let Some(size) = target.len().checked_add(text.len()) else {
        *issue = Some("search row text exceeds bound".into());
        return;
    };
    if size > MAX_TEXT_BYTES {
        *issue = Some("search row text exceeds bound".into());
        return;
    }
    if target.try_reserve(text.len()).is_err() {
        *issue = Some("allocating bounded search row text".into());
        return;
    }
    target.push_str(text);
}

pub(super) fn flush_node(
    target: &mut String,
    saw_text: &mut bool,
    raw: &str,
    issue: &mut Option<String>,
) {
    let normalized = match normalize(raw) {
        Ok(value) => value,
        Err(error) => {
            *issue = Some(error.to_string());
            return;
        }
    };
    if normalized.is_empty() {
        return;
    }
    let separator = usize::from(*saw_text);
    let Some(growth) = separator.checked_add(normalized.len()) else {
        *issue = Some("search row text exceeds bound".into());
        return;
    };
    let Some(size) = target.len().checked_add(growth) else {
        *issue = Some("search row text exceeds bound".into());
        return;
    };
    if size > MAX_TEXT_BYTES || target.try_reserve(growth).is_err() {
        *issue = Some("search row text exceeds bound".into());
        return;
    }
    if separator != 0 {
        target.push(' ');
    }
    target.push_str(&normalized);
    *saw_text = true;
}
