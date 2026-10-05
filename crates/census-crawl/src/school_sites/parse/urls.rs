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

pub fn rank_link(href: &str, label: &str) -> u8 {
    let blob = format!("{href} {label}").to_ascii_lowercase();
    if blob.contains("coach") {
        return 0;
    }
    if blob.contains("athletic") {
        return 1;
    }
    if blob.contains("staff") || blob.contains("directory") || blob.contains("faculty") {
        return 2;
    }
    3
}

pub fn resolve_href(site: &str, href: &str) -> String {
    if href.starts_with("http") {
        href.to_string()
    } else {
        format!(
            "{}/{}",
            site.trim_end_matches('/'),
            href.trim_start_matches('/')
        )
    }
}
