//! robots.txt: the once-per-host rule fetch and the `*`-group parser (RFC 9309 patterns).

use super::{FetchError, Fetcher};
use futures::StreamExt;
use regex::Regex;
use std::time::Duration;
use tracing::debug;

/// Maximum size for a robots.txt body: enough for the largest real-world file with wide margin.
///
/// A robots.txt is plain text; 64 KiB is far more than any source has ever published. The cap
/// prevents a hostile body from consuming memory.
const ROBOTS_MAX_BODY: usize = 64 * 1024;

/// robots.txt rule set for one host origin.
#[derive(Debug, Default, Clone)]
pub(super) struct RobotsRules {
    /// The parsed rules, in file order.
    rules: Vec<Rule>,
    pub(super) crawl_delay: Option<Duration>,
    /// Whether a robots.txt was actually fetched for this host.
    ///
    /// `false` means "no file was fetched" (absent, failed, or refused to answer): the host is
    /// treated as if it has no rules, per RFC 9309 — every path is allowed.
    ///
    /// `true` means "we fetched something":
    /// - `rules.is_empty()` + `crawl_delay.is_some()` → a crawl-delay-only file (all paths allowed).
    /// - `rules` is non-empty → explicit Allow/Disallow rules apply.
    /// - `rules.is_empty()` + `crawl_delay.is_none()` → fetch completed but yielded nothing
    ///   (e.g., a body with no valid directives, or a server that returned a non-200 that we
    ///   couldn't classify). In this case, every path is still allowed, but we record that we
    ///   tried (so the crawl-delay from that body, if any, was applied).
    fetched: bool,
}

/// One `Allow`/`Disallow` rule: the pattern as published, and its compiled matcher.
///
/// RFC 9309 patterns are not prefixes: `*` matches any run of characters and a trailing `$` anchors
/// the path's end, so `Disallow: /*directory` (Bound publishes exactly that) refuses
/// `/ia/schools/albia/directory/new`. Matching on `starts_with` alone would walk straight through it.
#[derive(Debug, Clone)]
struct Rule {
    allow: bool,
    /// The pattern exactly as published, used for specificity.
    pattern: String,
    /// The compiled pattern: anchored at the start, `*` widened to `.*`, `$` honoured at the end.
    matcher: Regex,
}

impl Rule {
    /// Compile one published pattern. An unparsable pattern yields `None` and the rule is dropped:
    /// a rule set this process cannot represent must not silently become an allow or a deny.
    fn compile(allow: bool, pattern: &str) -> Option<Self> {
        let (core, anchored) = match pattern.strip_suffix('$') {
            Some(core) => (core, true),
            None => (pattern, false),
        };
        // Escape the literal pattern first, then widen the one metacharacter robots.txt defines.
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
    /// Whether `path` may be requested: the longest matching pattern decides, and `Allow` wins
    /// ties. A path no pattern matches is allowed.
    pub(super) fn allows(&self, path: &str) -> bool {
        if !self.fetched {
            return true; // absent robots.txt == allow
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
                    // Allow wins ties.
                    if rule.allow && !best_allow {
                        best = Some((len, true));
                    }
                }
                _ => best = Some((len, rule.allow)),
            }
        }
        best.map(|(_, allow)| allow).unwrap_or(true)
    }

    /// Whether a robots.txt was actually fetched for this host.
    ///
    ///  means no file was fetched (absent or transport failure): the host is treated
    /// as if it has no rules.  means a fetch happened — the result may contain rules,
    /// a crawl delay, or neither (e.g., a body with no valid directives).
    #[cfg(test)]
    pub(super) fn was_fetched(&self) -> bool {
        self.fetched
    }
}

impl Fetcher {
    /// Fetch the origin's robots rules, apply them, and cache the result.
    ///
    /// If the rule set is already cached, the cached copy is returned immediately.
    /// Otherwise the rule set is fetched through the same path as normal requests:
    /// it is paced by the host's gate, read under the body cap, and counted in request
    /// accounting. The fetch itself does not trigger another robots check (no recursion).
    ///
    /// Explicit status outcomes:
    /// - `200` → parse the body into rules.
    /// - `404` / `410` → no robots file; `fetched: false` (allow everything).
    /// - `401` / `403` → the host refuses this client; close the host (`Disallow: /`).
    /// - `429` → rate-limited; count as unknown (no rules, but recorded as fetched).
    /// - `5xx` → server error; count as unknown (no rules, but recorded as fetched).
    /// - Transport failure → unknown (no rules, but recorded as fetched).
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
            // A server that answers 401/403 for its own robots.txt is refusing this client outright.
            // Walking it anyway is the 403 storm this branch exists to prevent, and it makes a run's
            // verdicts depend on whether that fetch happened to be refused - which an audit gate
            // cannot be.
            Ok((401 | 403, _)) => parse_robots(REFUSAL_RULES),
            // A server that has no robots.txt or no longer serves it: per RFC 9309, the absence
            // of a file means "walk anything".
            Ok((404 | 410, _)) => RobotsRules {
                fetched: false,
                ..Default::default()
            },
            // Rate-limited, server error, or transport failure: we fetched the file but could not
            // read rules from it. Mark as fetched (not absent) so the host's pacing still applied,
            // but do not close the host — we simply don't know its rules. The caller receives an
            // unknown outcome: we tried, we just couldn't classify the result.
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

    /// Fetch a robots.txt body through the proper policy path: paced, capped, accounted.
    ///
    /// Unlike `fetch` (the public API), this does not recurse into robots checks. It uses the
    /// same internal fetch path as normal requests — host gate pacing, body cap, timeout, and
    /// request accounting — so it cannot bypass the politeness policy.
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

        // Read the body under the robots body cap.
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
                // Body too large — return with what we have; the caller will get an empty parse.
                break;
            }
            body.extend_from_slice(&chunk);
        }

        Ok((status, body))
    }
}

/// robots.txt parsing for the `*` and named user-agent groups, with RFC 9309 patterns.
///
/// Only the `*` group is honoured: our own product token never appears in published rule sets,
/// and RFC 9309 says a crawler with no matching group follows the wildcard group. Named groups
/// (e.g. `GPTBot`, `Googlebot`) are silently skipped — our client is not one of them.
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

/// What a host that refuses to state its rules is read as: closed to this client.
pub(crate) const REFUSAL_RULES: &str = "User-agent: *\nDisallow: /\n";

/// The longest crawl delay honoured from a fetched `robots.txt`, whatever the file claims.
///
/// A delay past this is a host asking not to be walked at all; expressing that as a ceiling keeps
/// the run's own budget intact instead of parking one host's lane for the rest of the run.
const MAX_CRAWL_DELAY: Duration = Duration::from_secs(3600);

/// A crawl delay a fetched body is allowed to ask for, or `None` when the value is unusable.
///
/// The value arrives in a body this process did not write, and `Duration::from_secs_f64` panics on a
/// non-finite or overflowing one — `inf` and `1e30` are two lines of a hostile `robots.txt` away,
/// and a panic here would kill the walk that fetched it.
fn bounded_crawl_delay(seconds: f64) -> Option<Duration> {
    Some(
        Duration::try_from_secs_f64(seconds)
            .ok()?
            .min(MAX_CRAWL_DELAY),
    )
}

#[cfg(test)]
mod tests;
