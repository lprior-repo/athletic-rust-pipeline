use url::Url;

use crate::request::same_site;

/// Maximum server redirects one capture follows before refusing the chain.
pub(super) const MAX_REDIRECT_HOPS: u8 = 5;

/// Decide whether a redirect hop stays inside the trust boundary of the
/// lane's bound origin. Unparsable hop URLs fail closed; the trust rule
/// itself lives with the request builders so the lane manager and the
/// service-layer refusal share one definition.
pub(super) fn hop_within_origin(hop: &str, origin: &Url) -> bool {
    let Ok(target) = Url::parse(hop) else {
        return false;
    };
    same_site(&target, origin)
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
