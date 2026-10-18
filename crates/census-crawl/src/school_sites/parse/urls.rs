pub(super) fn host_of(url: &str) -> Option<&str> {
    let without_scheme = url.split("://").nth(1).map_or(url, |rest| rest);
    without_scheme
        .split('/')
        .next()
        .filter(|host| !host.is_empty())
}

pub fn base_domain(host: &str) -> String {
    let lowered = host.to_ascii_lowercase();
    let parts: Vec<&str> = lowered.split('.').collect();
    let count = parts.len();
    if count >= 3 {
        let last = parts.get(count.saturating_sub(1)).map_or("", |part| *part);
        let second = parts.get(count.saturating_sub(2)).map_or("", |part| *part);
        if matches!(second, "k12" | "us" | "state" | "cc" | "edu") && last.len() == 2 {
            return parts
                .get(count.saturating_sub(3)..)
                .map_or_else(String::new, |tail| tail.join("."));
        }
        if last.len() == 2 && second.len() <= 3 {
            return parts
                .get(count.saturating_sub(3)..)
                .map_or_else(String::new, |tail| tail.join("."));
        }
    }
    parts
        .get(count.saturating_sub(2)..)
        .map_or_else(String::new, |tail| tail.join("."))
}

pub(super) fn sport_link(href: &str, label: &str) -> bool {
    [link_path(href), label].into_iter().any(|text| {
        let words = text
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty());
        words.clone().any(|word| {
            ["track", "xc", "tf", "crosscountry"]
                .iter()
                .any(|sport| word.eq_ignore_ascii_case(sport))
        }) || words.clone().zip(words.skip(1)).any(|(first, second)| {
            first.eq_ignore_ascii_case("cross") && second.eq_ignore_ascii_case("country")
        })
    })
}

pub(super) fn link_path(href: &str) -> &str {
    let location = href.split(['?', '#']).next().map_or("", |path| path);
    let authority = location
        .split_once("://")
        .map(|(_, rest)| rest)
        .or_else(|| location.strip_prefix("//"));
    authority.map_or(location, |rest| {
        rest.split_once('/').map_or("", |(_, path)| path)
    })
}

pub fn rank_link(href: &str, label: &str) -> u8 {
    if sport_link(href, label) {
        return 0;
    }
    let path = link_path(href);
    let blob = format!("{path} {label}").to_ascii_lowercase();
    if ["coach", "staff", "directory", "faculty", "contact"]
        .iter()
        .any(|word| blob.contains(word))
        || label.trim().eq_ignore_ascii_case("activities")
        || path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .is_some_and(|part| part.eq_ignore_ascii_case("activities"))
    {
        return 1;
    }
    if blob.contains("athletic") {
        return 2;
    }
    3
}

pub fn resolve_href(site: &str, href: &str) -> Result<String, url::ParseError> {
    url::Url::parse(site)
        .and_then(|base| base.join(href))
        .map(|url| url.to_string())
}

pub(super) fn unrelated_sport_link(href: &str, label: &str) -> bool {
    let path = link_path(href);
    unrelated_sport_text(path)
        || (unrelated_sport_text(label) && !generic_hub_path(path) && !sport_link("", label))
}

fn generic_hub_path(path: &str) -> bool {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .is_some_and(|part| {
            [
                "athletics", "activities", "staff-directory", "coaches", "contact",
                "staff", "directory", "faculty", "contact-us",
            ]
                .iter()
                .any(|hub| part.eq_ignore_ascii_case(hub))
        })
}

fn unrelated_sport_text(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .any(|word| {
            [
                "football", "basketball", "baseball", "softball", "volleyball",
                "soccer", "wrestling", "cheer", "cheerleading", "dance", "golf",
                "tennis", "swimming", "diving", "lacrosse", "hockey", "gymnastics",
                "bowling", "waterpolo", "badminton", "fencing", "rowing", "rugby",
            ]
            .iter()
            .any(|sport| word.eq_ignore_ascii_case(sport))
        })
}
