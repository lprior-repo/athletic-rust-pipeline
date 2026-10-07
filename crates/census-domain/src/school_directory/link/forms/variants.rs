use super::super::super::name::MatchForm;
use super::{is_number_token, STRUCTURAL_TOKENS};

const TRAILING_TOKENS: [&str; 18] = [
    "high",
    "school",
    "hs",
    "senior",
    "sr",
    "junior",
    "jr",
    "academy",
    "prep",
    "preparatory",
    "charter",
    "public",
    "magnet",
    "middle",
    "elementary",
    "the",
    "and",
    "of",
];

pub(super) fn variants_of(original: &MatchForm, base: &MatchForm) -> Vec<MatchForm> {
    let expanded = expand(base);
    let mut out: Vec<MatchForm> = Vec::with_capacity(3);
    push_variant(&mut out, original, &expanded);
    for candidate in [strip_trailing(&expanded), strip_structural(&expanded)]
        .into_iter()
        .flatten()
    {
        push_variant(&mut out, original, &candidate);
    }
    out
}

fn push_variant(out: &mut Vec<MatchForm>, original: &MatchForm, candidate: &MatchForm) {
    if candidate.is_empty() || candidate == original || out.contains(candidate) {
        return;
    }
    if candidate
        .as_str()
        .split(' ')
        .filter(|token| !token.is_empty())
        .all(is_number_token)
    {
        return;
    }
    out.push(candidate.clone());
}

fn expand(form: &MatchForm) -> MatchForm {
    let tokens: Vec<&str> = form
        .as_str()
        .split(' ')
        .filter(|token| !token.is_empty())
        .collect();
    let mut out: Vec<&str> = Vec::with_capacity(tokens.len().saturating_add(4));
    let mut index = 0;
    while let Some(token) = tokens.get(index).copied() {
        if token == "h" && tokens.get(index.saturating_add(1)) == Some(&"s") {
            out.extend_from_slice(&["high", "school"]);
            index = index.saturating_add(2);
            continue;
        }
        if token == "j"
            && tokens.get(index.saturating_add(1)) == Some(&"h")
            && tokens.get(index.saturating_add(2)) == Some(&"s")
        {
            out.extend_from_slice(&["junior", "high", "school"]);
            index = index.saturating_add(3);
            continue;
        }
        let replacement: &[&str] = match token {
            "hs" => &["high", "school"],
            "jhs" => &["junior", "high", "school"],
            "sh" => &["senior", "high"],
            "sr" => &["senior"],
            "jr" => &["junior"],
            "mhs" | "chs" => &[],
            "acad" => &["academy"],
            "elem" => &["elementary"],
            "mid" => &["middle"],
            _ => &[token],
        };
        out.extend_from_slice(replacement);
        index = index.saturating_add(1);
    }
    MatchForm::of(&out.join(" "))
}

pub(super) fn directory_numbered_suffix(name: &str) -> bool {
    let trimmed = name.trim_end();
    let digits = trimmed
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .count();
    if !matches!(digits, 1..=2) {
        return false;
    }
    let Some(head) = trimmed.get(..trimmed.len().saturating_sub(digits)) else {
        return false;
    };
    head.trim_end()
        .chars()
        .last()
        .is_some_and(|last| !last.is_alphanumeric() && !last.is_whitespace())
}

pub(super) fn strip_numeric_tail(form: &MatchForm) -> MatchForm {
    let mut tokens: Vec<&str> = form
        .as_str()
        .split(' ')
        .filter(|token| !token.is_empty())
        .collect();
    if tokens
        .last()
        .is_some_and(|last| last.len() <= 2 && last.bytes().all(|byte| byte.is_ascii_digit()))
    {
        tokens.pop();
    }
    MatchForm::of(&tokens.join(" "))
}

fn strip_trailing(form: &MatchForm) -> Option<MatchForm> {
    let tokens: Vec<&str> = form
        .as_str()
        .split(' ')
        .filter(|token| !token.is_empty())
        .collect();
    let trailing = tokens
        .iter()
        .rev()
        .take_while(|token| TRAILING_TOKENS.contains(token))
        .count();
    let end = tokens.len().saturating_sub(trailing);
    let core: Vec<&str> = tokens
        .iter()
        .take(end)
        .copied()
        .filter(|token| *token != "the")
        .collect();
    if core.is_empty() {
        return None;
    }
    let core = MatchForm::of(&core.join(" "));
    if core == *form {
        None
    } else {
        Some(core)
    }
}

fn strip_structural(form: &MatchForm) -> Option<MatchForm> {
    let core: Vec<&str> = form
        .as_str()
        .split(' ')
        .filter(|token| !token.is_empty() && !STRUCTURAL_TOKENS.contains(token))
        .collect();
    if core.is_empty() {
        return None;
    }
    let core = MatchForm::of(&core.join(" "));
    if core == *form {
        None
    } else {
        Some(core)
    }
}
