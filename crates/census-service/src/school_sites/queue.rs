use census_crawl::school_sites::QueueRow;
use census_domain::UsJurisdiction;
use serde::Serialize;
use std::path::Path;

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
}

pub(super) fn plan(
    rows: Vec<QueueRow>,
    state_override: Option<UsJurisdiction>,
    selection: (Option<usize>, Option<usize>),
    stats: &mut QueueStats,
) -> anyhow::Result<Vec<PlannedSite>> {
    let mut seen = std::collections::BTreeSet::new();
    let mut sites =
        rows.into_iter()
            .try_fold(Vec::new(), |mut sites, row| -> anyhow::Result<_> {
                let Some(site) = admit(row, state_override, stats)? else {
                    return Ok(sites);
                };
                if !seen.insert((site.state, site.school.to_lowercase())) {
                    bump(&mut stats.duplicate)?;
                    return Ok(sites);
                }
                sites.try_reserve(1)?;
                sites.push(site);
                Ok(sites)
            })?;
    if let Some(sample) = selection.0.filter(|sample| *sample > 0) {
        let step = sites
            .len()
            .checked_div(sample)
            .map_or(1, |step| step.max(1));
        sites = sites.into_iter().step_by(step).take(sample).collect();
    } else if selection.0 == Some(0) {
        sites.clear();
    }
    if let Some(limit) = selection.1 {
        sites.truncate(limit);
    }
    Ok(sites)
}

fn admit(
    row: QueueRow,
    state_override: Option<UsJurisdiction>,
    stats: &mut QueueStats,
) -> anyhow::Result<Option<PlannedSite>> {
    bump(&mut stats.read)?;
    anyhow::ensure!(
        row.name.len() <= 512 && row.website.len() <= 4096 && row.state.len() <= 128,
        "queue field exceeds its admission budget"
    );
    if row.website.trim().is_empty() {
        bump(&mut stats.without_website)?;
        return Ok(None);
    }
    let state = if row.state.trim().is_empty() {
        state_override
    } else {
        UsJurisdiction::parse(&row.state)
    };
    let Some(state) = state else {
        bump(&mut stats.unmapped_state)?;
        return Ok(None);
    };
    let website = normalize_site(row.website.trim());
    Ok(Some(PlannedSite {
        state,
        school: row.name.trim().to_owned(),
        website,
    }))
}

fn bump(count: &mut usize) -> anyhow::Result<()> {
    *count = count
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("queue counter overflow"))?;
    Ok(())
}

pub(super) fn read_queue(path: &Path) -> anyhow::Result<(Vec<QueueRow>, usize)> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|error| anyhow::anyhow!("reading queue {}: {error}", path.display()))?;
    let size = file.metadata()?.len();
    anyhow::ensure!(size <= 8 * 1024 * 1024, "queue exceeds 8 MiB");
    let mut text = String::new();
    text.try_reserve_exact(usize::try_from(size)?)?;
    (&mut file).take(size).read_to_string(&mut text)?;
    anyhow::ensure!(
        u64::try_from(text.len())? == size,
        "queue changed while being read"
    );
    match file.read_exact(&mut [0_u8; 1]) {
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {}
        Err(error) => return Err(error.into()),
        Ok(()) => anyhow::bail!("queue changed while being read"),
    }
    let parsed = census_crawl::school_sites::parse_queue(&text);
    anyhow::ensure!(parsed.0.len() <= 65_536, "queue exceeds 65536 rows");
    Ok(parsed)
}

pub fn host_roots(queue_path: &Path) -> anyhow::Result<Vec<String>> {
    let (rows, _) = read_queue(queue_path)?;
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
