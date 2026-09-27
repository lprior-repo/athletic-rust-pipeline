
use crate::milesplit::fetch::fetch_meet_result_files;
use crate::net::{FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;

pub const MISMATCH_LIMIT: usize = 25;

const MILESPLIT_HOST: &str = "milesplit.com";
const MILESPLIT_HOST_SUFFIX: &str = ".milesplit.com";

pub fn is_results_page(url: &str) -> bool {
    let Some(host) = url.split('/').nth(2) else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    if host != MILESPLIT_HOST && !host.ends_with(MILESPLIT_HOST_SUFFIX) {
        return false;
    }
    let named = url
        .split('/')
        .any(|segment| segment.eq_ignore_ascii_case("meets"));
    let last = url
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .is_some_and(|segment| segment.eq_ignore_ascii_case("results"));
    named && last
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetPage {
    pub results_url: String,
    pub jurisdiction: UsJurisdiction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedResultFile {
    pub url: String,
    pub jurisdiction: UsJurisdiction,
    pub is_meet_pro: i64,
}

impl ListedResultFile {
    pub fn request(&self) -> super::ResultSetRequest {
        super::ResultSetRequest {
            url: self.url.clone(),
            jurisdiction: self.jurisdiction,
        }
    }
}

#[derive(Debug, Default)]
pub struct MeetPages {
    pub files: Vec<ListedResultFile>,
    pub pages_read: usize,
    pub quarantined: Vec<(String, String)>,
}

impl MeetPages {
    pub fn stopped(&self) -> bool {
        self.quarantined.len() >= MISMATCH_LIMIT && self.quarantined.len() > self.pages_read
    }
}

pub async fn read_meet_pages(
    fetcher: &Fetcher,
    pages: impl IntoIterator<Item = MeetPage>,
    options: &FetchOptions,
) -> CrawlResult<MeetPages> {
    let mut out = MeetPages::default();
    for page in pages {
        if out.stopped() {
            break;
        }
        match fetch_meet_result_files(fetcher, &page.results_url, options).await {
            Ok(files) => {
                out.pages_read = out.pages_read.saturating_add(1);
                out.files.extend(files.iter().map(|file| ListedResultFile {
                    url: file.raw_url(&page.results_url),
                    jurisdiction: page.jurisdiction,
                    is_meet_pro: file.is_meet_pro,
                }));
            }
            Err(CrawlError::Schema { detail, .. }) => {
                out.quarantined.push((page.results_url.clone(), detail));
            }
            Err(other) => return Err(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
