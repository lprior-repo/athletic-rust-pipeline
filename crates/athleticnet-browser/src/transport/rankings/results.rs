use super::rankings_helper::{declared_page, page_extent};
use crate::clock::Clock;
use crate::protocol::{RankingPageObservation, RankingsCapture};
use crate::request::{self, RankingsAction};
use crate::{gate::ProfileGate, BrowserError, BrowserResponse};
use chromiumoxide::Page;
use std::sync::Arc;
use std::time::Duration;

pub(super) async fn fetch_results(
    page: &Page,
    action: &RankingsAction,
    request_timeout: Duration,
    gate: Arc<ProfileGate>,
    source_origin: &url::Url,
    clock: &dyn Clock,
) -> Result<BrowserResponse, BrowserError> {
    let request = request::rankings_spec(source_origin, action.clone())
        .map_err(|_| BrowserError::Protocol)?;
    let body = match request.body() {
        Some(body) => serde_json::to_string(&body).map_err(|_| BrowserError::Protocol)?,
        None => return Err(BrowserError::Protocol),
    };
    let mut response = super::super::fetch(page, &request, request_timeout, gate, clock).await?;
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

pub(super) fn next_page_after(body: &[u8], page: u32) -> Option<u32> {
    if page > 1 && declared_page(body) == Some(1) {
        return None;
    }
    match page_extent(body) {
        Err(_) => None,
        Ok((0, _)) => None,
        Ok((rows, Some(depth))) if rows < depth => None,
        Ok((_, _)) => page.checked_add(1),
    }
}
