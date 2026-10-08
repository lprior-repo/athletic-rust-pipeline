use super::{FetchError, Fetcher};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::debug;

const ROBOTS_MAX_BODY: usize = 64 * 1024;

#[derive(Debug, Default, Clone)]
pub(super) struct RobotsPolicy {
    pub(super) crawl_delay: Option<Duration>,
}

impl Fetcher {
    pub(super) async fn robots_for(&self, scheme_host: &str, host: &str) -> RobotsPolicy {
        let single = {
            let mut gates = self.robots_gates.lock().await;
            gates
                .entry(scheme_host.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _single = single.lock().await;
        {
            let robots = self.robots.lock().await;
            if let Some(policy) = robots.get(scheme_host) {
                return policy.clone();
            }
        }
        let gate = self.host_gate(host, None).await;
        let _permit = gate.lock().await;
        self.wait_turn(host).await;
        let url = format!("{scheme_host}/robots.txt");
        let policy = match self.fetch_robots(&url).await {
            Ok((200, body)) => parse_robots(&String::from_utf8_lossy(&body)),
            Ok((status, _)) => {
                debug!(
                    status,
                    "robots.txt published no readable policy, continuing unpaced"
                );
                RobotsPolicy::default()
            }
            Err(e) => {
                debug!(
                    error = %e,
                    "robots.txt unavailable, continuing unpaced"
                );
                RobotsPolicy::default()
            }
        };
        debug!(
            host = scheme_host,
            crawl_delay = ?policy.crawl_delay,
            "robots pacing loaded"
        );
        self.robots
            .lock()
            .await
            .insert(scheme_host.to_string(), policy.clone());
        policy
    }

    async fn fetch_robots(&self, url: &str) -> Result<(u16, Vec<u8>), FetchError> {
        {
            let mut stats = self.stats.lock().await;
            stats.requests = stats.requests.saturating_add(1);
            let entry = stats.per_host.entry(super::host_of(url)).or_default();
            entry.requests = entry.requests.saturating_add(1);
        }
        let response =
            self.client
                .get(url)
                .send()
                .await
                .map_err(|source| FetchError::Transport {
                    url: url.to_string(),
                    source,
                })?;
        let status = response.status().as_u16();

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) =
            stream
                .next()
                .await
                .transpose()
                .map_err(|source| FetchError::Transport {
                    url: url.to_string(),
                    source,
                })?
        {
            if body.len().saturating_add(chunk.len()) > ROBOTS_MAX_BODY {
                break;
            }
            body.extend_from_slice(&chunk);
        }

        Ok((status, body))
    }
}

pub(crate) fn parse_robots(body: &str) -> RobotsPolicy {
    let mut crawl_delay = None;
    let mut applies = false;

    for line in body.lines() {
        let line = line.split('#').next().map_or("", |value| value).trim();
        if line.is_empty() {
            continue;
        }
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        let field = field.trim().to_ascii_lowercase();
        let value = value.trim();
        match field.as_str() {
            "user-agent" => applies = value == "*",
            "crawl-delay" if applies => {
                if let Ok(seconds) = value.parse::<f64>() {
                    crawl_delay = bounded_crawl_delay(seconds);
                }
            }
            _ => {}
        }
    }
    RobotsPolicy { crawl_delay }
}

const MAX_CRAWL_DELAY: Duration = Duration::from_secs(3600);

fn bounded_crawl_delay(seconds: f64) -> Option<Duration> {
    Some(
        Duration::try_from_secs_f64(seconds)
            .ok()?
            .min(MAX_CRAWL_DELAY),
    )
}

#[cfg(test)]
mod tests;
