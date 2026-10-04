use crate::UsJurisdiction;
use std::collections::BTreeMap;

use super::address::PostalAddress;
use super::entry::SchoolDirectoryEntry;
use super::key::{DirectoryKey, IdentifiedKey};
use super::label::SourceLabel;
use super::name::MatchForm;
use super::school::GradeSpan;

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
const MAX_CANDIDATES: usize = 8;
const MAX_ALIASES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkRule {
    ExactName,
    CoreName,
    Parenthetical,
    ParentheticalInner,
    Alias,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRef {
    pub key: IdentifiedKey,
    pub source: SourceLabel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkMatch {
    pub key: IdentifiedKey,
    pub source: SourceLabel,
    pub address: PostalAddress,
    pub website: Option<String>,
    pub rule: LinkRule,
    pub matched_form: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewReason {
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkDecision {
    Linked(Box<LinkMatch>),
    Review {
        reason: ReviewReason,
        candidates: Vec<CandidateRef>,
    },
    NoMatch,
}

struct IndexEntry {
    key: IdentifiedKey,
    source: SourceLabel,
    city: Option<MatchForm>,
    state: UsJurisdiction,
    address: PostalAddress,
    website: Option<String>,
    middle: bool,
}

pub struct DirectoryIndex {
    entries: Vec<IndexEntry>,
    by_name: BTreeMap<MatchForm, Vec<u32>>,
    by_core: BTreeMap<MatchForm, Vec<u32>>,
}

struct Form {
    rule: LinkRule,
    form: MatchForm,
}

impl DirectoryIndex {
    pub fn build(entries: &[SchoolDirectoryEntry]) -> Self {
        let mut index = Self {
            entries: Vec::with_capacity(entries.len()),
            by_name: BTreeMap::new(),
            by_core: BTreeMap::new(),
        };
        for entry in entries {
            let Some((key, source)) = identity(entry) else {
                continue;
            };
            let Some(address) = entry.address() else {
                continue;
            };
            if address.line1().is_none() {
                continue;
            }
            let Some(state) = address.state() else {
                continue;
            };
            let Some(name) = entry.name() else {
                continue;
            };
            let form = MatchForm::of(name.as_str());
            if form.is_empty() {
                continue;
            }
            let middle = is_middle(&form, entry.grades());
            let city = address
                .city()
                .map(|city| MatchForm::of(city.as_str()))
                .filter(|form| !form.is_empty());
            let Ok(position) = u32::try_from(index.entries.len()) else {
                continue;
            };
            index.entries.push(IndexEntry {
                key,
                source,
                city,
                state,
                address: address.clone(),
                website: entry.website().map(|website| website.as_str().to_string()),
                middle,
            });
            index
                .by_name
                .entry(form.clone())
                .or_default()
                .push(position);
            for variant in variants(&form) {
                index.by_core.entry(variant).or_default().push(position);
            }
        }
        index
    }

    pub fn link(
        &self,
        school_name: &str,
        normalized_name: &str,
        city: Option<&str>,
        state: UsJurisdiction,
        aliases: &[String],
    ) -> LinkDecision {
        let primary = MatchForm::of(school_name);
        let normalized = MatchForm::of(normalized_name);
        let middle = census_is_middle(&primary) || census_is_middle(&normalized);
        let city_form = city.map(MatchForm::of).filter(|form| !form.is_empty());
        let forms = candidate_forms(school_name, &primary, &normalized, aliases);
        let mut ambiguous: Option<Vec<CandidateRef>> = None;
        for form in &forms {
            let candidates = self.select(&form.form, state, middle);
            if candidates.is_empty() {
                continue;
            }
            let preferred = prefer_city(candidates, city_form.as_ref());
            let distinct = distinct_candidates(&preferred);
            if let [single] = distinct.as_slice() {
                let Some(first) = preferred.first().copied() else {
                    continue;
                };
                return LinkDecision::Linked(Box::new(LinkMatch {
                    key: single.key.clone(),
                    source: single.source.clone(),
                    address: first.address.clone(),
                    website: first.website.clone(),
                    rule: form.rule,
                    matched_form: form.form.as_str().to_string(),
                }));
            }
            if ambiguous.is_none() {
                ambiguous = Some(distinct);
            }
        }
        if let Some(candidates) = ambiguous {
            return LinkDecision::Review {
                reason: ReviewReason::Ambiguous,
                candidates,
            };
        }
        LinkDecision::NoMatch
    }

    fn select<'a>(
        &'a self,
        form: &MatchForm,
        state: UsJurisdiction,
        middle: bool,
    ) -> Vec<&'a IndexEntry> {
        let mut out: Vec<&IndexEntry> = Vec::new();
        for map in [&self.by_name, &self.by_core] {
            let Some(positions) = map.get(form) else {
                continue;
            };
            for &position in positions {
                let Some(entry) = self.entries.get(position as usize) else {
                    continue;
                };
                if entry.state != state || entry.middle != middle {
                    continue;
                }
                out.push(entry);
            }
        }
        out
    }
}

fn identity(entry: &SchoolDirectoryEntry) -> Option<(IdentifiedKey, SourceLabel)> {
    let key = match entry.key() {
        DirectoryKey::Nces(id) => IdentifiedKey::Nces(id.clone()),
        DirectoryKey::Pss(id) => IdentifiedKey::Pss(id.clone()),
        DirectoryKey::StateRecord { .. } | DirectoryKey::Weak(_) => return None,
    };
    let source = if entry.sources().contains(&SourceLabel::Ccd) {
        SourceLabel::Ccd
    } else if entry.sources().contains(&SourceLabel::Pss) {
        SourceLabel::Pss
    } else {
        return None;
    };
    Some((key, source))
}

fn candidate_forms(
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
        let head = MatchForm::of(head);
        if !head.is_empty() {
            push_form(&mut forms, LinkRule::Parenthetical, &head);
            push_variants(&mut forms, LinkRule::Parenthetical, &head);
        }
        let inner = MatchForm::of(inner);
        if !inner.is_empty() && token_count(&inner) >= 2 {
            push_form(&mut forms, LinkRule::ParentheticalInner, &inner);
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

fn variants(form: &MatchForm) -> Vec<MatchForm> {
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
    let mut out: Vec<&str> = Vec::with_capacity(tokens.len() + 4);
    let mut index = 0;
    while let Some(token) = tokens.get(index).copied() {
        if token == "h" && tokens.get(index + 1) == Some(&"s") {
            out.extend_from_slice(&["high", "school"]);
            index += 2;
            continue;
        }
        if token == "j"
            && tokens.get(index + 1) == Some(&"h")
            && tokens.get(index + 2) == Some(&"s")
        {
            out.extend_from_slice(&["junior", "high", "school"]);
            index += 3;
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
        index += 1;
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
    let mut end = tokens.len();
    while end > 0 && TRAILING_TOKENS.contains(&tokens[end - 1]) {
        end -= 1;
    }
    let core: Vec<&str> = tokens[..end]
        .iter()
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
    if !trimmed.ends_with(')') {
        return None;
    }
    let open = trimmed.rfind('(')?;
    let inner = trimmed[open + 1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return None;
    }
    Some((trimmed[..open].trim_end(), inner))
}

fn token_count(form: &MatchForm) -> usize {
    form.as_str()
        .split(' ')
        .filter(|token| !token.is_empty())
        .count()
}

fn prefer_city<'a>(
    candidates: Vec<&'a IndexEntry>,
    city: Option<&MatchForm>,
) -> Vec<&'a IndexEntry> {
    let Some(city) = city else {
        return candidates;
    };
    let filtered: Vec<&IndexEntry> = candidates
        .iter()
        .copied()
        .filter(|entry| entry.city.as_ref() == Some(city))
        .collect();
    if filtered.is_empty() {
        candidates
    } else {
        filtered
    }
}

fn census_is_middle(form: &MatchForm) -> bool {
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

fn is_middle(form: &MatchForm, grades: Option<GradeSpan>) -> bool {
    let grade_middle = grades.is_some_and(|span| span.high().rank().is_some_and(|rank| rank <= 10));
    census_is_middle(form) || grade_middle
}

fn distinct_candidates(candidates: &[&IndexEntry]) -> Vec<CandidateRef> {
    let mut out: Vec<CandidateRef> = Vec::new();
    for entry in candidates {
        if out.iter().any(|candidate| candidate.key == entry.key) {
            continue;
        }
        if out.len() >= MAX_CANDIDATES {
            break;
        }
        out.push(CandidateRef {
            key: entry.key.clone(),
            source: entry.source.clone(),
        });
    }
    out
}
