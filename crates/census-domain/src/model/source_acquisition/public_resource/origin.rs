use super::super::scalar;
use super::CaptureDomainError;

pub(super) fn validate(value: &str) -> Result<(), CaptureDomainError> {
    let parsed = scalar::url(value)?;
    if parsed.scheme() != "https"
        || parsed.port().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.origin().ascii_serialization() != value
    {
        return Err(CaptureDomainError::Invalid {
            field: "public source origin",
        });
    }
    match parsed.host() {
        Some(url::Host::Domain(host)) => validate_host(host),
        _ => Err(CaptureDomainError::Invalid {
            field: "public source host",
        }),
    }
}

fn validate_host(host: &str) -> Result<(), CaptureDomainError> {
    scalar::bounded(host, "public source host", 253)?;
    if !host.contains('.')
        || host.ends_with('.')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
        || forbidden_suffix(host)
    {
        return Err(CaptureDomainError::Invalid {
            field: "public source host",
        });
    }
    Ok(())
}

fn forbidden_suffix(host: &str) -> bool {
    [
        "localhost",
        "local",
        "localdomain",
        "internal",
        "home",
        "home.arpa",
        "lan",
        "intranet",
        "corp",
        "private",
        "invalid",
        "test",
        "example",
        "onion",
        "athletic.net",
    ]
    .iter()
    .any(|suffix| {
        host == *suffix
            || host
                .strip_suffix(*suffix)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}
