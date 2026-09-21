//! Whitespace-collapsing, ASCII-lowercasing text normalization shared by
//! event-key derivation, context fields, and equipment descriptors.

pub(super) fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
