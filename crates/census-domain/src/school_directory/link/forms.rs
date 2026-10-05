use super::super::name::MatchForm;
use super::super::school::GradeSpan;
use super::{Form, LinkRule, MAX_ALIASES};

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
const CAMPUS_TOKENS: [&str; 12] = [
    "east",
    "west",
    "north",
    "south",
    "central",
    "main",
    "upper",
    "lower",
    "northeast",
    "northwest",
    "southeast",
    "southwest",
];
const STRUCTURAL_TOKENS: [&str; 21] = [
    "high",
    "school",
    "hs",
    "senior",
    "sr",
    "junior",
    "jr",
    "jhs",
    "sh",
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
    "highschool",
];

pub(super) fn candidate_forms(
    school_name: &str,
    primary: &MatchForm,
    normalized: &MatchForm,
    aliases: &[String],
) -> Vec<Form> {
    let mut forms: Vec<Form> = Vec::with_capacity(MAX_ALIASES + 8);
    push_form(&mut forms, LinkRule::ExactName, primary);
    push_form(&mut forms, LinkRule::ExactName, normalized);
    push_variants(&mut forms, LinkRule::CoreName, primary);
    push_variants(&mut forms, LinkRule::CoreName, normalized);
    if let Some((head, inner)) = split_parenthetical(school_name) {
        let inner = MatchForm::of(inner);
        if !campus_designation(&inner) {
            let head = MatchForm::of(head);
            if !head.is_empty() {
                push_form(&mut forms, LinkRule::Parenthetical, &head);
                push_variants(&mut forms, LinkRule::Parenthetical, &head);
            }
            if !inner.is_empty() && token_count(&inner) >= 2 {
                push_form(&mut forms, LinkRule::ParentheticalInner, &inner);
            }
        }
    }
    for alias in aliases.iter().take(MAX_ALIASES) {
        let alias = MatchForm::of(alias);
        if alias.is_empty() {
            continue;
        }
        push_form(&mut forms, LinkRule::Alias, &alias);
        push_variants(&mut forms, LinkRule::Alias, &alias);
    }
    forms
}

fn push_form(forms: &mut Vec<Form>, rule: LinkRule, form: &MatchForm) {
    if form.is_empty() || forms.iter().any(|existing| existing.form == *form) {
        return;
    }
    forms.push(Form {
        rule,
        form: form.clone(),
    });
}

fn push_variants(forms: &mut Vec<Form>, rule: LinkRule, form: &MatchForm) {
    for variant in variants(form) {
        push_form(forms, rule, &variant);
    }
}

pub(super) fn variants(form: &MatchForm) -> Vec<MatchForm> {
    let expanded = expand(&strip_numeric_tail(form));
    let mut out: Vec<MatchForm> = Vec::with_capacity(3);
    push_variant(&mut out, form, &expanded);
    for candidate in [strip_trailing(&expanded), strip_structural(&expanded)]
        .into_iter()
        .flatten()
    {
        push_variant(&mut out, form, &candidate);
    }
    out
}

fn push_variant(out: &mut Vec<MatchForm>, original: &MatchForm, candidate: &MatchForm) {
    if candidate.is_empty() || candidate == original || out.contains(candidate) {
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

fn strip_numeric_tail(form: &MatchForm) -> MatchForm {
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

fn split_parenthetical(raw: &str) -> Option<(&str, &str)> {
    let trimmed = raw.trim_end();
    let body = trimmed.strip_suffix(')')?;
    let open = body.rfind('(')?;
    let (head, tail) = body.split_at(open);
    let inner = tail.strip_prefix('(')?.trim();
    if inner.is_empty() {
        return None;
    }
    Some((head.trim_end(), inner))
}

fn token_count(form: &MatchForm) -> usize {
    form.as_str()
        .split(' ')
        .filter(|token| !token.is_empty())
        .count()
}

fn campus_designation(inner: &MatchForm) -> bool {
    let mut designated = false;
    for token in inner.as_str().split(' ').filter(|token| !token.is_empty()) {
        if CAMPUS_TOKENS.contains(&token) {
            designated = true;
        } else if token != "campus" {
            return false;
        }
    }
    designated
}

pub(super) fn census_is_middle(form: &MatchForm) -> bool {
    let mut tokens: Vec<&str> = form
        .as_str()
        .split(' ')
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.last() == Some(&"school") {
        tokens.pop();
    }
    matches!(
        tokens.last().copied(),
        Some("middle" | "jhs" | "mid" | "elementary")
    ) || tokens.ends_with(&["junior", "high"])
        || tokens.ends_with(&["jr", "high"])
        || tokens.ends_with(&["j", "h", "s"])
}

pub(super) fn is_middle(form: &MatchForm, grades: Option<GradeSpan>) -> bool {
    let grade_middle = grades.is_some_and(|span| span.high().rank().is_some_and(|rank| rank <= 10));
    census_is_middle(form) || grade_middle
}
