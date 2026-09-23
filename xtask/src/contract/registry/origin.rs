//! The admission-origin syntax clause of check 5: what a descriptor's `origin` string may be.
//!
//! Split out so the registry module holds only what reads the registry: this is a pure predicate over
//! a string, it is where the labels-versus-IP-literal distinction lives, and it is the part of the
//! checks a test can drive without a registry behind it.

/// The longest a DNS label may be (RFC 1035).
const LABEL_MAX: usize = 63;

/// Whether an admission origin names a host: dot-separated labels, or a bracketed IP literal.
///
/// The declared value is a host, not a URL, so a scheme, a port, a path or a space is rejected; a
/// trailing root dot is accepted, as DNS itself accepts it. `local-artifact` — the origin an adapter
/// that contacts no host declares — is a single label, and passes, because a source that fetches
/// nothing still names the origin its policy is stated for.
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
    let name = origin.strip_suffix('.').unwrap_or(origin);
    !name.is_empty() && name.split('.').all(is_label)
}

/// Whether one DNS label is well formed: 1..=63 letters, digits or inner hyphens.
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
