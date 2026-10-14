use super::FetchError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use url::{Host, Url};

mod origin_policy;

pub(super) struct DestinationGuard {
    authorized_hosts: Vec<String>,
    #[cfg(test)]
    pub(super) fixture_resolution: Option<(String, std::net::SocketAddr)>,
}

impl DestinationGuard {
    pub(super) fn new(authorized_hosts: Vec<String>) -> Self {
        Self {
            authorized_hosts,
            #[cfg(test)]
            fixture_resolution: None,
        }
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

    #[tracing::instrument(skip(self))]
    pub(super) async fn validate_resolved_url(&self, raw: &str) -> Result<(), FetchError> {
        self.validate_url(raw)?;
        let parsed = Url::parse(raw).map_err(|source| FetchError::InvalidUrl {
            url: raw.to_string(),
            source,
        })?;
        let host = parsed
            .host_str()
            .ok_or_else(|| policy("source URL has no host"))?;
        let port = parsed
            .port_or_known_default()
            .ok_or_else(|| policy("source URL has no HTTP port"))?;
        #[cfg(test)]
        if let Some((name, address)) = &self.fixture_resolution {
            if host == name {
                return self.validate_ip(host, address.ip());
            }
        }
        let addresses = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            tokio::net::lookup_host((host, port)),
        )
        .await
        .map_err(|_| FetchError::Timeout {
            url: raw.to_string(),
            timeout_secs: 15,
        })?
        .map_err(|error| policy(&format!("source DNS lookup failed for {host}: {error}")))?;
        let mut count = 0_usize;
        for address in addresses {
            count = count
                .checked_add(1)
                .ok_or_else(|| policy("source DNS result count overflow"))?;
            if count > 64 {
                return Err(policy("source DNS lookup exceeds 64 addresses"));
            }
            self.validate_ip(host, address.ip())?;
        }
        if count == 0 {
            return Err(policy("source DNS lookup returned no addresses"));
        }
        Ok(())
    }

    pub(super) fn validate_ip(&self, host: &str, ip: IpAddr) -> Result<(), FetchError> {
        #[cfg(test)]
        if self
            .fixture_resolution
            .as_ref()
            .is_some_and(|(name, address)| name == host && address.ip() == ip)
        {
            return Ok(());
        }
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
            #[cfg(test)]
            if let Some((name, address)) = &guard.fixture_resolution {
                if host == name {
                    guard.validate_ip(host, address.ip())?;
                    let addresses: reqwest::dns::Addrs = Box::new(std::iter::once(*address));
                    return Ok(addresses);
                }
            }
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
mod wiring_tests;
