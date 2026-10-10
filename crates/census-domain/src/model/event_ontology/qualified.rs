use super::{LABEL_PHRASES, LABEL_QUALIFIERS};

const MODIFIERS: &[&str] = &[
    "relay",
    "medley",
    "hurdles",
    "hurdle",
    "steeplechase",
    "walk",
];

struct Token<'a> {
    text: &'a str,
    cleaned: String,
}

pub(super) fn candidates(label: &str) -> Vec<String> {
    let tokens: Vec<Token<'_>> = label
        .split_whitespace()
        .map(|text| Token {
            text,
            cleaned: text
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|ch| ch.to_ascii_lowercase())
                .collect(),
        })
        .collect();
    let kept: Vec<&Token<'_>> = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let before = index.checked_sub(1).and_then(|i| tokens.get(i));
            let after = index.checked_add(1).and_then(|i| tokens.get(i));
            let phrase = LABEL_PHRASES.iter().any(|(left, right)| {
                (token.cleaned == *left && after.is_some_and(|next| next.cleaned == *right))
                    || (token.cleaned == *right && before.is_some_and(|prev| prev.cleaned == *left))
            });
            (!phrase
                && !LABEL_QUALIFIERS.contains(&token.cleaned.as_str())
                && !division_code(token.text))
            .then_some(token)
        })
        .collect();
    let modifier_ranges: Vec<(usize, usize)> = kept
        .iter()
        .enumerate()
        .filter(|(_, token)| MODIFIERS.contains(&token.cleaned.as_str()))
        .map(|(index, _)| (index, index.saturating_add(1)))
        .collect();
    let mut candidates = Vec::new();
    if !kept.is_empty() {
        candidates.push(render(&kept));
    }
    for width in (1..=kept.len().min(4)).rev() {
        for (start, window) in kept.windows(width).enumerate() {
            if width == kept.len() {
                continue;
            }
            let compact_len = window.iter().fold(0_usize, |len, token| {
                len.saturating_add(token.cleaned.len())
            });
            let discards_modifier = modifier_ranges
                .iter()
                .any(|(first, after)| *first < start || *after > start.saturating_add(width));
            if compact_len >= 3 && !discards_modifier {
                candidates.push(render(window));
            }
        }
    }
    candidates
}

fn render(tokens: &[&Token<'_>]) -> String {
    let capacity = tokens.iter().fold(0_usize, |len, token| {
        len.saturating_add(token.text.len()).saturating_add(1)
    });
    let mut text = String::with_capacity(capacity);
    for token in tokens {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(token.text);
    }
    text
}

fn division_code(token: &str) -> bool {
    fn code(part: &str) -> bool {
        match part.as_bytes() {
            [first, second] => {
                (first.is_ascii_digit() && matches!(second, b'a'..=b'f'))
                    || (*first == b'd' && second.is_ascii_digit())
            }
            _ => false,
        }
    }
    let lower = token.to_ascii_lowercase();
    match lower.split_once('-') {
        Some((head, tail)) => code(head) && code(tail),
        None => code(&lower),
    }
}
