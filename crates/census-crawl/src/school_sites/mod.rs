use serde::{Deserialize, Serialize};

mod contacts;
pub mod parse;
mod run;
pub use contacts::{apply_contact_capture, collect_contacts};
pub(crate) use contacts::{persist as persist_contact_prefix, retain_contact_attempt};
pub use run::crawl_site;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use parse::{analyse, AdHit, CoachHit, Rules, Signals};

pub const SOURCE_ID: &str = "school_sites";
pub const MAX_SITE_PAGES: usize = 7;
pub const MAX_GUESS_PAGES: usize = 10;
pub const MAX_SEARCH_PAGES: usize = 12;
pub const MAX_SEARCH_FOLLOW: usize = 14;
pub const MIN_GUESS_BODY: usize = 200;
pub const GUESS_PATHS: [&str; 3] = ["/athletics", "/coaches", "/staff-directory"];
pub const SEARCH_QUERIES: [&str; 2] = ["cross+country+coach", "track+coach"];
pub const WORDPRESS_MARKERS: [&str; 3] = ["wp-content", "wp-json", "wp-includes"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueRow {
    #[serde(default)]
    pub state: String,
    pub name: String,
    pub website: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageEvidence {
    pub url: String,
    pub digest: String,
    pub fetched_at: String,
    pub status: u16,
}

#[derive(Debug, Clone)]
pub struct SiteOutcome {
    pub signals: Signals,
    pub requests: usize,
    pub errors: usize,
    pub note: Option<String>,
    pub pages: Vec<PageEvidence>,
}

pub fn parse_queue(text: &str) -> (Vec<QueueRow>, usize) {
    let mut rows = Vec::new();
    let mut rejected = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<QueueRow>(line) {
            Ok(row) => rows.push(row),
            Err(_) => rejected = rejected.saturating_add(1),
        }
    }
    (rows, rejected)
}

pub fn queue_key(state: &str, name: &str) -> String {
    format!("{}__{}", state.to_ascii_uppercase(), slug(name))
}

pub fn slug(name: &str) -> String {
    let lowered = name.to_ascii_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut pending_dash = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch);
        } else {
            pending_dash = true;
        }
    }
    out.truncate(60);
    out
}

pub fn site_root(site: &str) -> String {
    match site.split_once("://") {
        Some((scheme, rest)) => {
            let host = rest.split_once('/').map_or(rest, |(host, _)| host);
            format!("{scheme}://{host}")
        }
        None => site.to_string(),
    }
}
