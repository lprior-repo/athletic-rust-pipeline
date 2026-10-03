use super::FetchError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use url::{Host, Url};

mod origin_policy;

pub(super) struct DestinationGuard {
    authorized_hosts: Vec<String>,
}

impl DestinationGuard {
    pub(super) fn new(authorized_hosts: Vec<String>) -> Self {
        Self { authorized_hosts }
    }

    #[cfg(test)]
    pub(super) fn is_authorized(&self, host: &str) -> bool {
        origin_policy::host_granted(host, &self.authorized_hosts)
    }

    pub(super) fn permits_redirect(&self, original: &Url, destination: &Url) -> bool {
        origin_policy::permits_redirect(original, destination, &self.authorized_hosts)
    }

    pub(super) fn validate_url(&self, raw: &str) -> Result<(), FetchError> {
        let url = Url::parse(raw).map_err(|source| FetchError::InvalidUrl {
            url: raw.to_string(),
            source,
        })?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(policy("source URLs require HTTP(S) without credentials"));
        }
        match url.host() {
            Some(Host::Ipv4(ip)) => self.validate_ip(&ip.to_string(), IpAddr::V4(ip)),
            Some(Host::Ipv6(ip)) => self.validate_ip(&ip.to_string(), IpAddr::V6(ip)),
            Some(Host::Domain(host)) if host.eq_ignore_ascii_case("localhost") => {
                self.validate_ip(host, IpAddr::V4(Ipv4Addr::LOCALHOST))
            }
            Some(Host::Domain(_)) => Ok(()),
            None => Err(policy("source URL has no host")),
        }
    }

    pub(super) fn validate_ip(&self, host: &str, ip: IpAddr) -> Result<(), FetchError> {
        let public = match ip {
            IpAddr::V4(ip) => public_v4(ip),
            IpAddr::V6(ip) => public_v6(ip),
        };
        let explicit_local = self.authorized_hosts.iter().any(|allowed| {
            (host == "localhost" && allowed == "localhost" && ip.is_loopback())
                || (host.parse::<IpAddr>().ok() == Some(ip)
                    && allowed.parse::<IpAddr>().ok() == Some(ip))
        });
        if public || explicit_local {
            Ok(())
        } else {
            Err(policy(&format!(
                "non-public destination {ip} for {host} is not explicitly authorized"
            )))
        }
    }

    pub(super) fn redirect(
        &self,
        attempt: reqwest::redirect::Attempt<'_>,
    ) -> reqwest::redirect::Action {
        if attempt.previous().len() >= 5 {
            return attempt.error("source redirect limit exceeded");
        }
        if let Err(error) = self.validate_url(attempt.url().as_str()) {
            return attempt.error(error);
        }
        let Some(original) = attempt.previous().first() else {
            return attempt.error("source redirect has no original URL");
        };
        if !self.permits_redirect(original, attempt.url()) {
            return attempt.error("source redirect destination is not authorized");
        }
        let Some(host) = attempt.url().host_str() else {
            return attempt.error("source redirect destination has no host");
        };
        if matches!(
            crate::registry::transport_for_host(host),
            Some(crate::registry::TransportKind::Browser)
        ) {
            return attempt.error("HTTP redirect destination requires browser transport");
        }
        attempt.follow()
    }
}

fn policy(detail: &str) -> FetchError {
    FetchError::Policy {
        detail: detail.to_string(),
    }
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !matches!(a, 0 | 10 | 127 | 224..=255)
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 169 && b == 254)
        && !(a == 172 && (16..=31).contains(&b))
        && !(a == 192 && (b == 168 || (b == 0 && matches!(c, 0 | 2))))
        && !(a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        && !(a == 203 && b == 0 && c == 113)
}

fn public_v6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return public_v4(mapped);
    }
    let [a, b, ..] = ip.segments();
    (a & 0xe000) == 0x2000
        && a != 0x2002
        && !(a == 0x2001 && (b <= 0x01ff || b == 0x0db8))
        && !(a == 0x3fff && b <= 0x0fff)
}

pub(super) struct GuardedResolver(pub(super) Arc<DestinationGuard>);

impl reqwest::dns::Resolve for GuardedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let guard = Arc::clone(&self.0);
        Box::pin(async move {
            let host = name.as_str();
            let addresses = tokio::net::lookup_host((host, 0)).await?;
            let mut validated = Vec::new();
            for address in addresses {
                guard.validate_ip(host, address.ip())?;
                validated.push(address);
            }
            if validated.is_empty() {
                return Err(policy("source DNS lookup returned no addresses").into());
            }
            let addresses: reqwest::dns::Addrs = Box::new(validated.into_iter());
            Ok(addresses)
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod redirect_tests;

#[cfg(test)]
mod wiring_tests;
