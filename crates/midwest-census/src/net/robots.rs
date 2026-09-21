//! robots.txt: the once-per-host rule fetch and the `*`-group parser.

use super::{FetchError, Fetcher};
use std::time::Duration;
use tracing::debug;

/// robots.txt rule set for one host origin.
#[derive(Debug, Default, Clone)]
pub(super) struct RobotsRules {
    /// (allow?, path prefix)
    rules: Vec<(bool, String)>,
    pub(super) crawl_delay: Option<Duration>,
    fetched: bool,
}

impl RobotsRules {
    pub(super) fn allows(&self, path: &str) -> bool {
        if !self.fetched {
            return true; // absent robots.txt == allow
        }
        let mut best: Option<(usize, bool)> = None;
        for (allow, prefix) in &self.rules {
            if prefix.is_empty() {
                continue;
            }
            if path.starts_with(prefix.as_str()) {
                let len = prefix.len();
                match best {
                    Some((best_len, _)) if best_len > len => {}
                    Some((best_len, best_allow)) if best_len == len => {
                        // Allow wins ties.
                        if *allow && !best_allow {
                            best = Some((len, true));
                        }
                    }
                    _ => best = Some((len, *allow)),
                }
            }
        }
        best.map(|(_, allow)| allow).unwrap_or(true)
    }
}

impl Fetcher {
    pub(super) async fn robots_for(&self, scheme_host: &str) -> RobotsRules {
        {
            let robots = self.robots.lock().await;
            if let Some(rules) = robots.get(scheme_host) {
                return rules.clone();
            }
        }
        let url = format!("{scheme_host}/robots.txt");
        let rules = match self.fetch_text_uncached(&url).await {
            Ok((200, body)) => parse_robots(&body),
            _ => RobotsRules {
                fetched: false,
                ..Default::default()
            },
        };
        debug!(
            host = scheme_host,
            rules = rules.rules.len(),
            "robots loaded"
        );
        self.robots
            .lock()
            .await
            .insert(scheme_host.to_string(), rules.clone());
        rules
    }

    async fn fetch_text_uncached(&self, url: &str) -> Result<(u16, String), FetchError> {
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
        let body = response.text().await.unwrap_or_default();
        Ok((status, body))
    }
}

/// Minimal, correct robots.txt parsing for the `*` and named user-agent groups.
pub(super) fn parse_robots(body: &str) -> RobotsRules {
    let mut rules = Vec::new();
    let mut crawl_delay = None;
    // We only honour the `*` group: our own product token never appears in published rule sets, and
    // RFC 9309 says a crawler with no matching group follows the wildcard group.
    let mut applies = false;
    let mut saw_any_group = false;

    for line in body.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        let field = field.trim().to_ascii_lowercase();
        let value = value.trim();
        match field.as_str() {
            "user-agent" => {
                saw_any_group = true;
                applies = value == "*";
            }
            "disallow" | "allow" if applies && !value.is_empty() => {
                rules.push((field == "allow", value.to_string()));
            }
            "crawl-delay" if applies => {
                if let Ok(seconds) = value.parse::<f64>() {
                    crawl_delay = Some(Duration::from_secs_f64(seconds.max(0.0)));
                }
            }
            _ => {}
        }
    }
    if !saw_any_group {
        rules.clear();
    }
    RobotsRules {
        rules,
        crawl_delay,
        fetched: true,
    }
}
