use super::rankings_helper::{declared_page, page_extent};
use crate::runtime::browser::{gate::ProfileGate, BrowserError, BrowserResponse};
use crate::runtime::protocol::{RankingPageObservation, RankingsCapture};
use crate::runtime::source::request::{self, RankingsAction};
use chromiumoxide::Page;
use std::sync::Arc;
use std::time::Duration;

/// Serve a `Results` capture through the persistent in-page fetch lane: the
/// physical request is the site's own rankings API POST, issued from the
/// bootstrapped source page, so cookies, TLS, and fingerprint stay in Chromium.
/// The semantic (UI listing) URL and the receipt identity are unchanged.
pub(super) async fn fetch_results(
    page: &Page,
    action: &RankingsAction,
    request_timeout: Duration,
    gate: Arc<ProfileGate>,
    source_origin: &url::Url,
) -> Result<BrowserResponse, BrowserError> {
    let request = request::rankings_spec(source_origin, action.clone())
        .map_err(|_| BrowserError::Protocol)?;
    let body = match request.body() {
        Some(body) => serde_json::to_string(&body).map_err(|_| BrowserError::Protocol)?,
        None => return Err(BrowserError::Protocol),
    };
    let mut response = super::super::fetch(page, &request, request_timeout, gate).await?;
    let next_page = next_page_after(&response.body, action.page);
    response.rankings = Some(RankingPageObservation {
        capture: RankingsCapture::Results,
        request_method: "POST".to_owned(),
        request_url: request.url.as_str().to_owned(),
        request_body: Some(body),
        next_page,
    });
    Ok(response)
}

/// Pagination follows the payload's own declared page depth: only a page whose
/// rows fill `settings.depth` can have a successor, and a page carrying fewer
/// rows is the last page of the list. Live measurement (2026-09-20, USA indoor
/// division 173005, boys grade 11) fixed the shape: a complete `100m` list
/// returned 71 rows against `depth: 100` while `200m` returned a full 100 plus
/// the blurred tail row, and the source answered a `page=2` request with a
/// byte-identical page-1 payload (`settings.page` stayed 1). Requesting a
/// successor for a short page therefore cannot retrieve new evidence and would
/// only raise a page-identity conflict, so the chain ends on the short page.
/// The same declared page-1 answer is the source's response to a request past
/// the listing's last page (measured live on the USA outdoor division 168416,
/// boys grade 11 `100m`: the app's own request carried `page=246` and the
/// capture declared `settings.page: 1`). A list whose final page fills the
/// declared depth would otherwise request a successor that never exists, so a
/// deep request answered with the listing's first page terminates the chain
/// instead of advancing. Publication seals that terminating page into the
/// event's page count, so verification requires exactly `None` on the head page
/// and `Some(page + 1)` on every page before it.
pub(super) fn next_page_after(body: &[u8], page: u32) -> Option<u32> {
    if page > 1 && declared_page(body) == Some(1) {
        return None;
    }
    match page_extent(body) {
        // Malformed pages end pagination: strict publication parsing rejects
        // them before a checkpoint is written.
        Err(_) => None,
        Ok((0, _)) => None,
        Ok((rows, Some(depth))) if rows < depth => None,
        Ok((_, _)) => page.checked_add(1),
    }
}
