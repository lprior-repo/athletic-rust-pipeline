use super::{FetchError, Fetcher};
use futures::StreamExt;
use regex::Regex;
use std::time::Duration;
use tracing::debug;

const ROBOTS_MAX_BODY: usize = 64 * 1024;

#[derive(Debug, Default, Clone)]
pub(super) struct RobotsRules {
    rules: Vec<Rule>,
    pub(super) crawl_delay: Option<Duration>,
    fetched: bool,
}

#[derive(Debug, Clone)]
struct Rule {
    allow: bool,
    pattern: String,
    matcher: Regex,
}

impl Rule {
    fn compile(allow: bool, pattern: &str) -> Option<Self> {
        let (core, anchored) = match pattern.strip_suffix('$') {
            Some(core) => (core, true),
            None => (pattern, false),
        };
        let mut source = String::with_capacity(core.len().saturating_add(4));
        source.push('^');
        source.push_str(&regex::escape(core).replace(r"\*", ".*"));
        if anchored {
            source.push('$');
        }
        Regex::new(&source).ok().map(|matcher| Self {
            allow,
            pattern: pattern.to_string(),
            matcher,
        })
    }
}

impl RobotsRules {
    pub(super) fn allows(&self, path: &str) -> bool {
        if !self.fetched {
            return true;
        }
        let mut best: Option<(usize, bool)> = None;
        for rule in &self.rules {
            if !rule.matcher.is_match(path) {
                continue;
            }
            let len = rule.pattern.len();
            match best {
                Some((best_len, _)) if best_len > len => {}
                Some((best_len, best_allow)) if best_len == len => {
                    if rule.allow && !best_allow {
                        best = Some((len, true));
                    }
                }
                _ => best = Some((len, rule.allow)),
            }
        }
        best.map(|(_, allow)| allow).unwrap_or(true)
    }

    #[cfg(test)]
    pub(super) fn was_fetched(&self) -> bool {
        self.fetched
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
        let rules = match self.fetch_robots(&url).await {
            Ok((200, body)) => parse_robots(&String::from_utf8_lossy(&body)),
            Ok((401 | 403, _)) => parse_robots(REFUSAL_RULES),
            Ok((404 | 410, _)) => RobotsRules {
                fetched: false,
                ..Default::default()
            },
            Ok((status, _)) => {
                debug!(
                    status,
                    "robots fetch returned non-standard status, allowing all"
                );
                RobotsRules {
                    fetched: true,
                    rules: Vec::new(),
                    crawl_delay: None,
                }
            }
            Err(_) => {
                debug!("robots fetch failed, allowing all (transport/server error)");
                RobotsRules {
                    fetched: true,
                    rules: Vec::new(),
                    crawl_delay: None,
                }
            }
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

    async fn fetch_robots(&self, url: &str) -> Result<(u16, Vec<u8>), FetchError> {
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

pub(crate) fn parse_robots(body: &str) -> RobotsRules {
    let mut rules = Vec::new();
    let mut crawl_delay = None;
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
                if let Some(rule) = Rule::compile(field == "allow", value) {
                    rules.push(rule);
                }
            }
            "crawl-delay" if applies => {
                if let Ok(seconds) = value.parse::<f64>() {
                    crawl_delay = bounded_crawl_delay(seconds);
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

pub(crate) const REFUSAL_RULES: &str = "User-agent: *\nDisallow: /\n";

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
