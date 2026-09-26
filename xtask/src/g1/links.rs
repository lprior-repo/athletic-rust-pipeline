//! Athlete-href classification: one href in, one class token out.

use regex::Regex;

/// Site-absolute prefixes an athlete href may carry before its path.
static ABS_PREFIXES: &[&str] = &[
    "https://www.athletic.net",
    "https://athletic.net",
    "http://www.athletic.net",
    "http://athletic.net",
];

/// The path part of an href: no site-absolute prefix, no query, no fragment.
pub(crate) fn path_of(href: &str) -> &str {
    let mut value = href;
    for prefix in ABS_PREFIXES {
        if value.to_lowercase().starts_with(prefix) {
            if let Some(tail) = value.get(prefix.len()..) {
                value = tail;
            }
            break;
        }
    }
    let value = value.split('?').next().unwrap_or(value);
    value.split('#').next().unwrap_or(value)
}

/// How one href classifies: a class token, plus the sport token for the canonical sported forms.
pub(crate) fn link_class(
    href: &str,
    athlete_re: &Regex,
    sported_re: &Regex,
    sportless_re: &Regex,
    empty_id_re: &Regex,
) -> (&'static str, Option<&'static str>) {
    if !athlete_re.is_match(href) {
        return ("not_athlete", None);
    }
    let path = path_of(href);
    if let Some(caps) = sported_re.captures(path) {
        let sport = caps
            .get(2)
            .map(|g| g.as_str().to_lowercase())
            .unwrap_or_default();
        let token = if sport == "track-and-field" {
            "tf"
        } else {
            "xc"
        };
        return ("sported", Some(token));
    }
    if sportless_re.is_match(path) {
        return ("sportless", None);
    }
    if empty_id_re.is_match(path) {
        return ("empty_id", None);
    }
    ("other_noncanonical", None)
}
