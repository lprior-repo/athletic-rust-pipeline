//! Reading a run of meet pages without letting one malformed page end the walk.
//!
//! §62: one bad source object must not terminate the census. The results arm reads a meet's page to
//! learn which result files it lists, and a page whose template this build does not recognise is a
//! page it cannot read — not a reason to abandon the hundreds of meets around it. The unrecognised
//! page is recorded with its URL and the parser's own reason, and the walk continues.
//!
//! Only a schema mismatch is quarantined. A transport failure propagates, because §9 requires that a
//! source failure never be read as absence: a page this run could not *reach* is a fact about the
//! run, not a fact about the meet.
//!
//! A template that has genuinely changed shows up as a run of quarantines rather than as a silent
//! skip, and [`MeetPages::stopped`] turns a majority-mismatch run into a stopped walk instead of an
//! unbounded stream of requests spent learning the same thing (§69, repeated malformed contract).

use crate::milesplit::fetch::fetch_meet_result_files;
use crate::net::{FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;

/// Unrecognised pages a walk records before it will stop on them.
pub const MISMATCH_LIMIT: usize = 25;

/// MileSplit's host, and the same host as a subdomain suffix: every state site used to be its own
/// host (`oh.milesplit.com`), and both spellings name one family.
const MILESPLIT_HOST: &str = "milesplit.com";
const MILESPLIT_HOST_SUFFIX: &str = ".milesplit.com";

/// Whether a row's URL is a meet's results page on one of MileSplit's state sites.
///
/// A shape check rather than a parse: the page itself carries no provider id, and the `/raw`
/// addresses under it are what [`crate::milesplit::ResultSetRef`] validates before a request. What
/// this decides is only whether a page belongs to the MileSplit reader at all — a WIAA artifact, a
/// Wayzata schedule or an Athletic.net meet is another provider's page, and fetching it here would
/// spend a request to read a template this arm does not know.
///
/// Both the durable results arm and the CLI's report arm read the same `source_meets` table, so the
/// decision lives here rather than in either caller.
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

/// A meet page to read: its results URL, and the jurisdiction its own `source_meets` row filed the
/// meet under.
///
/// The jurisdiction travels with the address for the reason the results route states: a page that
/// redirects to `www` publishes no state of its own, so the caller's row is the only thing that
/// knows whose meet it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetPage {
    pub results_url: String,
    pub jurisdiction: UsJurisdiction,
}

/// One result file a readable page listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedResultFile {
    /// The `/raw` address the result set is read from under the page that listed it.
    pub url: String,
    /// The jurisdiction of the meet whose page listed it.
    pub jurisdiction: UsJurisdiction,
    /// The page's own `isMeetPro` mark, carried through uninterpreted so a caller can decide for
    /// itself: the cost-aware report arm skips the files the platform serves through its paid path,
    /// while the census arm reads them and records whatever the host answers.
    pub is_meet_pro: i64,
}

impl ListedResultFile {
    /// The request this file becomes: the `/raw` address, and the jurisdiction of the row that named
    /// the meet.
    pub fn request(&self) -> super::ResultSetRequest {
        super::ResultSetRequest {
            url: self.url.clone(),
            jurisdiction: self.jurisdiction,
        }
    }
}

/// What reading a run of meet pages yielded.
#[derive(Debug, Default)]
pub struct MeetPages {
    /// One entry per result file of every page that was read, in meet order.
    pub files: Vec<ListedResultFile>,
    /// Pages read, whether or not they listed a result file.
    pub pages_read: usize,
    /// Pages this build could not read: `(results_url, reason)`, in meet order.
    pub quarantined: Vec<(String, String)>,
}

impl MeetPages {
    /// Whether the walk stopped early instead of reading every supplied page.
    ///
    /// A handful of unrecognised pages among readable ones is a source fact worth recording — sites
    /// carry junk meets. More mismatches than reads is not that: it is the template this build knows
    /// no longer being the one the site serves, which §69 stops a source's automatic work on.
    pub fn stopped(&self) -> bool {
        self.quarantined.len() >= MISMATCH_LIMIT && self.quarantined.len() > self.pages_read
    }
}

/// Read each meet page and collect the result files it lists, quarantining pages whose template this
/// build does not know.
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
