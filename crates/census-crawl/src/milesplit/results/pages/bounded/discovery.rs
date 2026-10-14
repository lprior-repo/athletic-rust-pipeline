use super::{lossy_body, raw_url, resource, Allocation, MAX_FILES, MAX_TEXT};
use crate::milesplit::results::pages::{ListedResultFile, MeetPage, MeetPages};
use crate::net::{FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};

#[tracing::instrument(skip_all)]
pub(super) async fn discover(
    fetcher: &Fetcher,
    pages: impl IntoIterator<Item = MeetPage>,
    options: &FetchOptions,
    allocation: &mut Allocation,
) -> CrawlResult<MeetPages> {
    let mut out = MeetPages::default();
    for (ordinal, page) in pages.into_iter().enumerate() {
        if out.stopped() {
            break;
        }
        if ordinal >= MAX_FILES {
            return Err(resource("meet pages", ordinal.saturating_add(1), MAX_FILES));
        }
        if page.results_url.len() > MAX_TEXT {
            return Err(resource("meet-page URL", page.results_url.len(), MAX_TEXT));
        }
        let outcome = fetcher.get(&page.results_url, options).await?;
        if outcome.status != 200 {
            return Err(crate::net::FetchError::Http {
                status: outcome.status,
                url: outcome.url,
            }
            .into());
        }
        let html = match std::str::from_utf8(&outcome.body) {
            Ok(html) => std::borrow::Cow::Borrowed(html),
            Err(_) => std::borrow::Cow::Owned(lossy_body(&outcome.body, allocation)?),
        };
        let remaining = MAX_FILES
            .checked_sub(out.files.len())
            .ok_or_else(|| resource("meet-page files", usize::MAX, MAX_FILES))?;
        match crate::milesplit::parse::meet_index::parse_bounded(
            &outcome.url,
            &html,
            remaining,
            allocation,
        ) {
            Ok(files) => {
                record_files(&mut out, &page, files, allocation)?;
            }
            Err(CrawlError::Schema { detail, .. }) => {
                if detail.len() > MAX_TEXT {
                    return Err(resource("meet-page diagnostic", detail.len(), MAX_TEXT));
                }
                allocation.push(&mut out.quarantined, (page.results_url, detail), MAX_FILES)?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(out)
}

fn record_files(
    out: &mut MeetPages,
    page: &MeetPage,
    files: Vec<crate::milesplit::MeetResultFile>,
    allocation: &mut Allocation,
) -> CrawlResult<()> {
    out.pages_read = out
        .pages_read
        .checked_add(1)
        .ok_or_else(|| resource("meet pages", usize::MAX, MAX_FILES))?;
    for file in files {
        let url = raw_url(&file, &page.results_url, allocation)?;
        allocation.push(
            &mut out.files,
            ListedResultFile {
                url,
                jurisdiction: page.jurisdiction,
                is_meet_pro: file.is_meet_pro,
            },
            MAX_FILES,
        )?;
    }
    Ok(())
}
