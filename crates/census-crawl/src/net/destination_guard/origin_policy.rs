use url::Url;

pub(super) fn permits_redirect(original: &Url, destination: &Url, grants: &[String]) -> bool {
    let Some(host) = destination.host_str() else {
        return false;
    };
    (original.scheme() == destination.scheme()
        && original.host_str() == Some(host)
        && original.port_or_known_default() == destination.port_or_known_default())
        || host_granted(host, grants)
}

pub(super) fn host_granted(host: &str, grants: &[String]) -> bool {
    let host = host.trim_matches(['[', ']']);
    grants.iter().any(|allowed| {
        host.eq_ignore_ascii_case(allowed)
            || host
                .len()
                .checked_sub(allowed.len())
                .and_then(|start| Some((host.get(..start)?, host.get(start..)?)))
                .is_some_and(|(prefix, suffix)| {
                    prefix.ends_with('.') && suffix.eq_ignore_ascii_case(allowed)
                })
    })
}

#[cfg(test)]
mod tests;
