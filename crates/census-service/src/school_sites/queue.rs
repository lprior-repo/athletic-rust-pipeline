use census_crawl::school_sites::{queue_key, QueueRow};
use census_domain::UsJurisdiction;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct QueueStats {
    pub read: usize,
    pub rejected: usize,
    pub without_website: usize,
    pub unmapped_state: usize,
    pub duplicate: usize,
}

#[derive(Debug, Clone)]
pub struct PlannedSite {
    pub state: UsJurisdiction,
    pub school: String,
    pub website: String,
    pub row: QueueRow,
    pub artifact: PathBuf,
    pub key: String,
}

impl PlannedSite {
    pub fn sort_key(&self) -> (String, String) {
        (self.state.code().to_string(), self.school.to_lowercase())
    }

    pub fn rows_path(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.csv", self.key))
    }
}

pub fn resumable(site: &PlannedSite, site_rows_dir: &Path, refresh: bool) -> bool {
    !refresh && site.artifact.exists() && site.rows_path(site_rows_dir).exists()
}

pub fn plan(
    rows: Vec<QueueRow>,
    state_override: Option<UsJurisdiction>,
    sample: Option<usize>,
    limit: Option<usize>,
    out_dir: &Path,
    stats: &mut QueueStats,
) -> Vec<PlannedSite> {
    let mut sites: Vec<PlannedSite> = Vec::new();
    let mut seen: std::collections::BTreeSet<(UsJurisdiction, String)> =
        std::collections::BTreeSet::new();
    for row in rows {
        stats.read = stats.read.saturating_add(1);
        let website = row.website.trim().to_string();
        if website.is_empty() {
            stats.without_website = stats.without_website.saturating_add(1);
            continue;
        }
        let state = match UsJurisdiction::parse(&row.state).or(state_override) {
            Some(state) => state,
            None => {
                stats.unmapped_state = stats.unmapped_state.saturating_add(1);
                continue;
            }
        };
        if !seen.insert((state, row.name.to_lowercase())) {
            stats.duplicate = stats.duplicate.saturating_add(1);
            continue;
        }
        let key = queue_key(state.code(), &row.name);
        let normalized = normalize_site(&website);
        let mut row = row;
        row.website = normalized.clone();
        sites.push(PlannedSite {
            state,
            school: row.name.trim().to_string(),
            website: normalized,
            row,
            artifact: out_dir.join(format!("{key}.json")),
            key,
        });
    }
    if let Some(sample) = sample.filter(|value| *value > 0) {
        let step = sites
            .len()
            .checked_div(sample)
            .map_or(1, |step| step.max(1));
        sites = sites.into_iter().step_by(step).take(sample).collect();
    } else if let Some(limit) = limit.filter(|value| *value > 0) {
        sites.truncate(limit);
    }
    sites
}

pub fn host_roots(queue_path: &Path) -> anyhow::Result<Vec<String>> {
    let text = std::fs::read_to_string(queue_path)
        .map_err(|error| anyhow::anyhow!("reading queue {}: {error}", queue_path.display()))?;
    let (rows, _) = census_crawl::school_sites::parse_queue(&text);
    let mut hosts: Vec<String> = rows
        .iter()
        .filter_map(|row| host_of(&normalize_site(row.website.trim())))
        .map(|host| census_crawl::school_sites::parse::base_domain(&host))
        .collect();
    hosts.sort();
    hosts.dedup();
    Ok(hosts)
}

fn host_of(site: &str) -> Option<String> {
    let rest = site.split_once("://").map_or(site, |(_, rest)| rest);
    let host = rest.split_once('/').map_or(rest, |(host, _)| host).trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

pub(crate) fn normalize_site(website: &str) -> String {
    if website.starts_with("http://") || website.starts_with("https://") {
        website.to_string()
    } else {
        format!("https://{website}")
    }
}
