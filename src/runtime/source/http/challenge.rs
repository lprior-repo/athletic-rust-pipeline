use reqwest::header::HeaderMap;

const MAX_CHALLENGE_SCAN_BYTES: usize = 128 * 1024;

pub(super) fn cf_header_challenge(headers: &HeaderMap) -> bool {
    headers
        .get_all("cf-mitigated")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|value| value.eq_ignore_ascii_case("challenge"))
}

pub(super) fn html_body_challenge(media_type: &str, body: &[u8]) -> bool {
    if !is_html_media_type(media_type) {
        return false;
    }
    let scan_len = body.len().min(MAX_CHALLENGE_SCAN_BYTES);
    let sample = match body.get(..scan_len) {
        Some(sample) => sample,
        None => return false,
    };
    let title = contains_ascii(sample, b"<title>just a moment");
    let platform = contains_ascii(sample, b"/cdn-cgi/challenge-platform/");
    let challenge_form = contains_ascii(sample, b"<form id=\"challenge-form")
        || contains_ascii(sample, b"<form id='challenge-form");
    let challenge_error = contains_ascii(sample, b"id=\"challenge-error-text\"")
        || contains_ascii(sample, b"id='challenge-error-text'");
    let instruction = contains_ascii(sample, b"enable javascript and cookies to continue");
    let challenge_config = contains_ascii(sample, b"window._cf_chl_opt");
    let secondary = challenge_form || challenge_error || instruction || challenge_config;
    (platform && (title || secondary))
        || (title && secondary)
        || (challenge_form && (challenge_error || instruction || challenge_config))
}

fn is_html_media_type(media_type: &str) -> bool {
    let base = media_type.split(';').next().map_or("", str::trim);
    base.eq_ignore_ascii_case("text/html") || base.eq_ignore_ascii_case("application/xhtml+xml")
}

fn contains_ascii(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloudflare_header_is_definitive() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cf-mitigated",
            reqwest::header::HeaderValue::from_static("ChAlLeNgE"),
        );
        assert!(cf_header_challenge(&headers));
    }

    #[test]
    fn html_challenge_requires_specific_conjunctive_markers() {
        let challenge = br#"<html><head><title>Just a moment...</title></head><body><script src="/cdn-cgi/challenge-platform/h/b"></script></body></html>"#;
        assert!(html_body_challenge("text/html; charset=utf-8", challenge));

        let public_page = br#"<html><script src="/cdn-cgi/challenge-platform/h/b"></script><div class="cookie-consent">Accept cookies</div><div class="signup-overlay">Sign up</div><div class="subscription-modal">Subscribe</div></html>"#;
        assert!(!html_body_challenge("text/html", public_page));
        assert!(!html_body_challenge("application/json", challenge));
    }

    #[test]
    fn passive_platform_script_alone_is_not_a_challenge() {
        let public_page = br#"<script src="/cdn-cgi/challenge-platform/h/b"></script>"#;
        assert!(!html_body_challenge("text/html", public_page));
    }

    #[test]
    fn challenge_markers_beyond_scan_bound_are_ignored() {
        let mut body = vec![b'x'; MAX_CHALLENGE_SCAN_BYTES];
        body.extend_from_slice(
            br#"<title>Just a moment...</title><script src="/cdn-cgi/challenge-platform/h/b"></script>"#,
        );
        assert!(!html_body_challenge("text/html", &body));
    }
}
