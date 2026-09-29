use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

pub const VARSITY_LEVEL: &str = "Varsity";
pub const UNSTATED_LEVEL: &str = "unstated";

static ROLE_SUFFIX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)[\s,]*[\(\[]?\s*(?:the\s+)?(?:assistant\s+|associate\s+|district\s+)?(?:athletic\s+director|director\s+of\s+athletics|head\s+coach|assistant\s+coach|coach)\s*[\)\]]?\s*$",
    )
});

static ROLE_ONLY: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:the\s+)?(?:assistant\s+|associate\s+|district\s+)?(?:athletic\s+director|director\s+of\s+athletics|head\s+coach|assistant\s+coach|coach|principal|superintendent|counselor|secretary|nurse|dean)$",
    )
});

static NON_COACH_LEAD: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:principal|superintendent|assistant\s+principal|counselor|secretary|nurse|registrar)\b",
    )
});

static DEAN_POST: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)^dean\b(?:\s+of\b|\s*[,:])"));

static VENDOR_SCHOOL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\btest\s+school\b"));

const VENDOR_EMAIL_SUFFIX: &str = "@dragonflyathletics.com";

fn role_suffix() -> CrawlResult<&'static Regex> {
    ROLE_SUFFIX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "role suffix",
            source: source.clone(),
        })
}

fn role_only() -> CrawlResult<&'static Regex> {
    ROLE_ONLY.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "role only",
        source: source.clone(),
    })
}

fn non_coach_lead() -> CrawlResult<&'static Regex> {
    NON_COACH_LEAD
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "non-coach lead",
            source: source.clone(),
        })
}

fn dean_post() -> CrawlResult<&'static Regex> {
    DEAN_POST.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "dean post",
        source: source.clone(),
    })
}

fn vendor_school() -> CrawlResult<&'static Regex> {
    VENDOR_SCHOOL
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "vendor school",
            source: source.clone(),
        })
}

fn is_python_space(ch: char) -> bool {
    ch.is_ascii_whitespace()
        || matches!(
            ch,
            '\u{1c}'..='\u{1f}'
                | '\u{85}'
                | '\u{a0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
        )
}

pub fn clean_text(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    let mut pending_space = false;
    for ch in value.chars() {
        if is_python_space(ch) {
            pending_space = !cleaned.is_empty();
        } else {
            if pending_space {
                cleaned.push(' ');
                pending_space = false;
            }
            cleaned.push(ch);
        }
    }
    cleaned
}

pub fn sanitize_person(value: &str) -> CrawlResult<Option<String>> {
    let cleaned = clean_text(value);
    let person = role_suffix()?
        .replace(&cleaned, "")
        .trim_matches(|ch: char| matches!(ch, ' ' | ',' | ';' | ':' | '-'))
        .to_string();
    if person.is_empty()
        || role_only()?.is_match(&person)
        || non_coach_lead()?.is_match(&person)
        || dean_post()?.is_match(&person)
    {
        return Ok(None);
    }
    Ok(Some(person))
}

pub fn is_vendor_school(value: &str) -> CrawlResult<bool> {
    Ok(vendor_school()?.is_match(value))
}

pub fn is_vendor_contact(email: &str) -> bool {
    email.to_ascii_lowercase().ends_with(VENDOR_EMAIL_SUFFIX)
}

pub fn is_vendor_fixture(school: &str, email: &str) -> CrawlResult<bool> {
    Ok(is_vendor_school(school)? || is_vendor_contact(email))
}

pub fn sanitize_school(value: &str) -> CrawlResult<Option<String>> {
    let cleaned = clean_text(value);
    if cleaned.is_empty() || is_vendor_school(&cleaned)? {
        return Ok(None);
    }
    Ok(Some(cleaned))
}

pub fn is_varsity_level(level: Option<&str>) -> bool {
    match level {
        None => true,
        Some(value) => value.is_empty() || value == VARSITY_LEVEL,
    }
}

pub fn level_label(level: Option<&str>) -> String {
    match level {
        Some(value) => {
            let cleaned = clean_text(value);
            if cleaned.is_empty() {
                UNSTATED_LEVEL.to_string()
            } else {
                cleaned
            }
        }
        None => UNSTATED_LEVEL.to_string(),
    }
}

#[cfg(test)]
#[path = "row_hygiene/tests.rs"]
mod tests;
