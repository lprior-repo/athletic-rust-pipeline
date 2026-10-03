const LABEL_MAX: usize = 63;

pub(super) fn is_host(origin: &str) -> bool {
    if let Some(literal) = origin
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return !literal.is_empty()
            && literal
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() || ch == ':' || ch == '.');
    }
    let name = origin
        .strip_suffix('.')
        .map_or(origin, core::convert::identity);
    !name.is_empty() && name.split('.').all(is_label)
}

fn is_label(label: &str) -> bool {
    !label.is_empty()
        && label.chars().count() <= LABEL_MAX
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::is_host;

    #[test]
    fn a_host_is_labels_and_nothing_else() {
        for host in [
            "www.athletic.net",
            "local-artifact",
            "kshsaa-api.kshsaa.org",
            "www.wayzataresults.com.",
            "127.0.0.1",
            "[2606:4700::1111]",
        ] {
            assert!(is_host(host), "{host} names a host");
        }
        for not_a_host in [
            "",
            " ",
            "www.athletic.net:443",
            "https://www.athletic.net",
            "www.athletic.net/path",
            "-lead.example",
            "trail-.example",
            "two words.example",
            "example..org",
            "[not:hex]",
            ".example",
        ] {
            assert!(!is_host(not_a_host), "{not_a_host} does not name a host");
        }
    }

    #[test]
    fn a_label_stops_at_sixty_three_characters() {
        let longest = format!("{}.example", "a".repeat(63));
        let too_long = format!("{}.example", "a".repeat(64));
        assert!(is_host(&longest));
        assert!(!is_host(&too_long));
    }
}
