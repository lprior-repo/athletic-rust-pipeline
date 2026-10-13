use super::{LABEL_PHRASES, LABEL_QUALIFIERS};

const MODIFIERS: &[&str] = &[
    "relay",
    "medley",
    "hurdles",
    "hurdle",
    "hurdling",
    "steeplechase",
    "walk",
    "walking",
    "racewalk",
    "lowhurdles",
    "highhurdles",
    "highhurdle",
    "lowhurdle",
    "steeple",
    "racewalking",
    "racewalker",
    "racewalkers",
    "wheelchair",
    "wheelchairs",
    "handcycle",
    "handcycles",
    "ambulatory",
    "paraambulatory",
    "para",
];

const MODIFIER_PHRASES: &[(&str, &str)] = &[
    ("race", "walk"),
    ("race", "walking"),
    ("race", "walker"),
    ("race", "walkers"),
    ("steeple", "chase"),
    ("steeple", "chasing"),
    ("low", "hurdle"),
    ("low", "hurdles"),
    ("high", "hurdle"),
    ("high", "hurdles"),
    ("wheel", "chair"),
    ("wheel", "chairs"),
    ("hand", "cycle"),
    ("hand", "cycles"),
    ("para", "ambulatory"),
];

struct Token<'a> {
    text: &'a str,
    cleaned: String,
}

pub(super) fn candidates(label: &str) -> Vec<String> {
    let tokens = tokenize(label);
    let kept = kept_tokens(&tokens);
    let modifier_ranges = modifier_runs(&modifier_flags(&kept));
    let mut candidates = Vec::new();
    if !kept.is_empty() {
        candidates.push(render(&kept));
    }
    window_candidates(&kept, &modifier_ranges, &mut candidates);
    candidates
}

fn tokenize(label: &str) -> Vec<Token<'_>> {
    label
        .split_whitespace()
        .map(|text| Token {
            text,
            cleaned: text
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|ch| ch.to_ascii_lowercase())
                .collect(),
        })
        .collect()
}

fn kept_tokens<'a>(tokens: &'a [Token<'a>]) -> Vec<&'a Token<'a>> {
    tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| contextual(tokens, index, token).then_some(token))
        .collect()
}

fn contextual(tokens: &[Token<'_>], index: usize, token: &Token<'_>) -> bool {
    let before = index.checked_sub(1).and_then(|i| tokens.get(i));
    let after = index.checked_add(1).and_then(|i| tokens.get(i));
    let phrase = LABEL_PHRASES.iter().any(|(left, right)| {
        (token.cleaned == *left && after.is_some_and(|next| next.cleaned == *right))
            || (token.cleaned == *right && before.is_some_and(|prev| prev.cleaned == *left))
    });
    !phrase && !LABEL_QUALIFIERS.contains(&token.cleaned.as_str()) && !division_code(token.text)
}

fn modifier_flags(kept: &[&Token<'_>]) -> Vec<bool> {
    kept.iter()
        .enumerate()
        .map(|(index, token)| modifier_at(kept, index, token))
        .collect()
}

fn modifier_at(kept: &[&Token<'_>], index: usize, token: &Token<'_>) -> bool {
    MODIFIERS.contains(&token.cleaned.as_str())
        || kept
            .get(index.saturating_add(1))
            .is_some_and(|next| phrase_pair(token, next))
        || index
            .checked_sub(1)
            .and_then(|previous| kept.get(previous))
            .is_some_and(|previous| phrase_pair(previous, token))
}

fn phrase_pair(left: &Token<'_>, right: &Token<'_>) -> bool {
    MODIFIER_PHRASES
        .iter()
        .any(|(modifier, head)| left.cleaned == *modifier && right.cleaned == *head)
}

fn window_candidates(
    kept: &[&Token<'_>],
    modifier_ranges: &[(usize, usize)],
    candidates: &mut Vec<String>,
) {
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
}

fn modifier_runs(flags: &[bool]) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for (index, flagged) in flags.iter().enumerate() {
        if !flagged {
            continue;
        }
        match ranges.last_mut() {
            Some((_, after)) if *after == index => *after = index.saturating_add(1),
            _ => ranges.push((index, index.saturating_add(1))),
        }
    }
    ranges
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
