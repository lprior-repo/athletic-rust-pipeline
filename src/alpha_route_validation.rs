use anyhow::{bail, Context, Result};

/// Validate a route by resolving against the base URL.
/// Ensures same scheme/host, rejects backslash authority escapes,
/// userinfo, query, and fragment.
pub fn validate_route(
    route: &str,
    base: &url::Url,
    allowed: &[String],
) -> Result<()> {
    // Reject network-path references (//host/path) — even same-host.
    if route.starts_with("//") {
        bail!(
            "api route '{}' must not begin with // (network-path reference)",
            route
        );
    }

    // Must be an absolute path starting with /.
    if !route.starts_with('/') {
        bail!(
            "api route '{}' must be an HTTPS-relative path starting with '/'",
            route
        );
    }

    // Reject backslash escapes.
    if route.contains('\\') {
        bail!("api route '{}' must not contain backslashes", route);
    }

    // Resolve route against the base URL.
    let resolved = base.join(route).with_context(|| {
        format!("api route '{}' is not valid relative to base URL", route)
    })?;

    // Must use HTTPS scheme.
    if resolved.scheme() != "https" {
        bail!("api route '{}' resolved to non-HTTPS scheme", route);
    }

    // Must resolve to the same host.
    if resolved.host_str() != base.host_str() {
        bail!(
            "api route '{}' resolves to a different host than the base URL",
            route
        );
    }

    // Reject routes with userinfo.
    if resolved.authority().contains('@') {
        bail!("api route '{}' must not contain userinfo", route);
    }

    // Reject routes with query or fragment.
    if resolved.query().is_some() || resolved.fragment().is_some() {
        bail!("api route '{}' must not contain query or fragment", route);
    }

    // Must be in allowed_routes.
    if !allowed.contains(&route.to_owned()) {
        bail!("api route '{}' is not in allowed_routes", route);
    }

    Ok(())
}
