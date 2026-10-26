use url::Url;

/// Maximum server redirects one capture follows before refusing the chain.
pub(super) const MAX_REDIRECT_HOPS: u8 = 5;

/// Decide whether a redirect hop stays inside the trust boundary of the
/// lane's bound origin.
///
/// The rule is deliberately conservative and needs no public-suffix list:
/// the hop keeps the origin's scheme and port, carries no userinfo, and
/// either names the origin host exactly or names a sibling subdomain under
/// the same parent domain (`al.milesplit.com` vs `www.milesplit.com` share
/// `milesplit.com`). Anything else fails closed: the caller refuses the
/// redirect exactly as if following were disabled.
pub(super) fn hop_within_origin(hop: &str, origin: &Url) -> bool {
    let target = match Url::parse(hop) {
        Ok(url) => url,
        Err(_) => return false,
    };
    if target.scheme() != origin.scheme() {
        return false;
    }
    if target.port_or_known_default() != origin.port_or_known_default() {
        return false;
    }
    if !target.username().is_empty() || target.password().is_some() {
        return false;
    }
    let target_host = match target.host_str() {
        Some(host) => host,
        None => return false,
    };
    let origin_host = match origin.host_str() {
        Some(host) => host,
        None => return false,
    };
    if target_host.eq_ignore_ascii_case(origin_host) {
        return true;
    }
    match (
        strip_first_label(target_host),
        strip_first_label(origin_host),
    ) {
        (Some(target_parent), Some(origin_parent)) => {
            target_parent.eq_ignore_ascii_case(origin_parent) && target_parent.contains('.')
        }
        _ => false,
    }
}

/// Strip the leftmost DNS label (`www.milesplit.com` becomes
/// `milesplit.com`). Returns `None` when there is no parent to name.
fn strip_first_label(host: &str) -> Option<&str> {
    let rest = host.split_once('.')?.1;
    (!rest.is_empty()).then_some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(raw: &str) -> Url {
        Url::parse(raw).expect("test origin parses")
    }

    #[test]
    fn exact_host_stays_inside() {
        assert!(hop_within_origin(
            "https://www.milesplit.com/athletes/1-a",
            &origin("https://www.milesplit.com")
        ));
    }

    #[test]
    fn sibling_state_subdomain_stays_inside() {
        assert!(hop_within_origin(
            "https://al.milesplit.com/athletes/1-a",
            &origin("https://www.milesplit.com")
        ));
    }

    #[test]
    fn foreign_host_fails_closed() {
        assert!(!hop_within_origin(
            "https://evil.example/athletes/1-a",
            &origin("https://www.milesplit.com")
        ));
    }

    #[test]
    fn lookalike_suffix_fails_closed() {
        assert!(!hop_within_origin(
            "https://www.milesplit.com.evil.example/athletes/1-a",
            &origin("https://www.milesplit.com")
        ));
    }

    #[test]
    fn bare_suffix_tld_fails_closed() {
        assert!(!hop_within_origin(
            "https://al.com/athletes/1-a",
            &origin("https://www.com")
        ));
    }

    #[test]
    fn deep_subdomain_fails_closed() {
        assert!(!hop_within_origin(
            "https://a.b.milesplit.com/athletes/1-a",
            &origin("https://www.milesplit.com")
        ));
    }

    #[test]
    fn scheme_downgrade_fails_closed() {
        assert!(!hop_within_origin(
            "http://al.milesplit.com/athletes/1-a",
            &origin("https://www.milesplit.com")
        ));
    }

    #[test]
    fn credentialed_hop_fails_closed() {
        assert!(!hop_within_origin(
            "https://lane:secret@al.milesplit.com/",
            &origin("https://www.milesplit.com")
        ));
    }

    #[test]
    fn unparsable_hop_fails_closed() {
        assert!(!hop_within_origin(
            "://not a url",
            &origin("https://www.milesplit.com")
        ));
    }
}
