/// Purpose-specific URL and state validation for alpha normalization.
use url::Url;

const ATHLETIC_NET_HOSTS: &[&str] = &["athletic.net", "www.athletic.net"];

/// Known US state two-letter codes and full names.
const KNOWN_STATES: &[(/* code */ &str, /* full */ &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
];
fn valid_origin(parsed: &Url, raw: &str) -> bool {
    let authority = raw
        .split_once("://")
        .map_or("", |(_, rest)| rest.split(['/', '?', '#']).next().map_or("", |part| part));
    parsed.scheme() == "https"
        && parsed.host_str().is_some()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
        && !authority.contains('@')
        && (parsed.port().is_none() || parsed.port() == Some(443))
}

/// Validate a profile URL: https, athletic.net host, /athlete/<nonzero-id>.
/// Rejects userinfo, query, fragment, token-bearing URLs.
pub fn validate_profile_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    if !valid_origin(&parsed, trimmed) {
        return None;
    }
    let host = parsed.host_str()?;
    if !ATHLETIC_NET_HOSTS.contains(&host) {
        return None;
    }
    let path = parsed.path();
    if !path.starts_with("/athlete/") {
        return None;
    }
    let id_str = path.strip_prefix("/athlete/")?;
    let id: u64 = id_str.parse().ok()?;
    if id == 0 {
        return None;
    }
    Some(format!("https://athletic.net/athlete/{id}"))
}

/// Validate a result URL: https, athletic.net host, /result/<id> route.
pub fn validate_result_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    if !valid_origin(&parsed, trimmed) {
        return None;
    }
    let host = parsed.host_str()?;
    if !ATHLETIC_NET_HOSTS.contains(&host) {
        return None;
    }
    let id_str = parsed.path().strip_prefix("/result/")?;
    let id: u64 = id_str.parse().ok()?;
    if id == 0 {
        return None;
    }
    Some(format!("https://athletic.net/result/{id}"))
}

/// Validate a source URL: https on athletic.net host.
pub fn validate_source_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    if !valid_origin(&parsed, trimmed) {
        return None;
    }
    let host = parsed.host_str()?;
    if !ATHLETIC_NET_HOSTS.contains(&host) {
        return None;
    }
    Some(format!("https://athletic.net{}", parsed.path()))
}

/// Canonicalize a state: accept known 50-state codes or full names, reject unknown.
/// Returns the uppercase two-letter code, or None.
pub fn canonical_state(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let upper = trimmed.to_uppercase();
    for (code, name) in KNOWN_STATES {
        if *code == upper {
            return Some(upper);
        }
        if name.to_lowercase() == trimmed.to_lowercase() {
            return Some(code.to_uppercase());
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_implicit_and_explicit_default_https_port() {
        let implicit = Url::parse("https://athletic.net/athlete/12345").expect("URL");
        assert!(valid_origin(&implicit, "https://athletic.net/athlete/12345"));
        assert_eq!(
            validate_profile_url("https://athletic.net/athlete/12345"),
            Some("https://athletic.net/athlete/12345".to_owned())
        );
        assert_eq!(
            validate_profile_url("https://athletic.net:443/athlete/12345"),
            Some("https://athletic.net/athlete/12345".to_owned())
        );
    }
}
