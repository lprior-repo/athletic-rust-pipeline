use regex::Regex;

static ABS_PREFIXES: &[&str] = &[
    "https://www.athletic.net",
    "https://athletic.net",
    "http://www.athletic.net",
    "http://athletic.net",
];

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
    let value = value
        .split('?')
        .next()
        .map_or(value, core::convert::identity);
    value
        .split('#')
        .next()
        .map_or(value, core::convert::identity)
}

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
            .map_or(Default::default(), core::convert::identity);
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
