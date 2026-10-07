use super::super::name::MatchForm;
use super::super::school::GradeSpan;
use super::{Form, LinkRule, MAX_ALIASES};
use variants::{directory_numbered_suffix, strip_numeric_tail, variants_of};

mod variants;

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
const CAMPUS_WORDS: [&str; 2] = ["campus", "campuses"];
pub(super) const STRUCTURAL_TOKENS: [&str; 21] = [
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
    push_name_variants(&mut forms, LinkRule::CoreName, school_name, primary);
    push_name_variants(&mut forms, LinkRule::CoreName, school_name, normalized);
    if let Some((head, inner)) = split_parenthetical(school_name) {
        if !campus_designation(inner) {
            let head_form = MatchForm::of(head);
            if !head_form.is_empty() {
                push_form(&mut forms, LinkRule::Parenthetical, &head_form);
                push_name_variants(&mut forms, LinkRule::Parenthetical, head, &head_form);
            }
            let inner_form = MatchForm::of(inner);
            if !inner_form.is_empty() && token_count(&inner_form) >= 2 {
                push_form(&mut forms, LinkRule::ParentheticalInner, &inner_form);
            }
        }
    }
    for alias in aliases.iter().take(MAX_ALIASES) {
        let form = MatchForm::of(alias);
        if form.is_empty() {
            continue;
        }
        push_form(&mut forms, LinkRule::Alias, &form);
        push_name_variants(&mut forms, LinkRule::Alias, alias.as_str(), &form);
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

fn push_name_variants(forms: &mut Vec<Form>, rule: LinkRule, name: &str, form: &MatchForm) {
    for variant in name_variants(name, form) {
        push_form(forms, rule, &variant);
    }
}

pub(super) fn name_variants(name: &str, form: &MatchForm) -> Vec<MatchForm> {
    if directory_numbered_suffix(name) {
        return variants_of(form, &strip_numeric_tail(form));
    }
    variants_of(form, form)
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

fn campus_designation(inner: &str) -> bool {
    let tokens: Vec<(&str, MatchForm)> = inner
        .split_whitespace()
        .map(|token| (token, MatchForm::of(token)))
        .collect();
    if tokens.is_empty() {
        return false;
    }
    if tokens
        .iter()
        .any(|(_, form)| CAMPUS_WORDS.contains(&form.as_str()))
    {
        return true;
    }
    if tokens
        .iter()
        .all(|(_, form)| CAMPUS_TOKENS.contains(&form.as_str()))
    {
        return true;
    }
    let numbers = tokens
        .iter()
        .filter(|(raw, _)| is_number_token(raw))
        .count();
    if numbers > 0 {
        return numbers < tokens.len()
            && tokens.iter().all(|(raw, form)| {
                is_number_token(raw)
                    || STRUCTURAL_TOKENS.contains(&form.as_str())
                    || CAMPUS_TOKENS.contains(&form.as_str())
            });
    }
    if tokens
        .iter()
        .any(|(raw, _)| raw.chars().any(|ch| !ch.is_alphanumeric()))
    {
        return false;
    }
    if !tokens
        .iter()
        .any(|(raw, _)| raw.chars().any(|ch| ch.is_lowercase()))
    {
        return false;
    }
    if tokens
        .iter()
        .all(|(_, form)| STRUCTURAL_TOKENS.contains(&form.as_str()))
    {
        return false;
    }
    true
}

pub(super) fn is_number_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit())
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
