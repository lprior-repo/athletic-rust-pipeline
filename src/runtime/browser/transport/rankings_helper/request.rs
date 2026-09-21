use super::capture::CapturedRanking;
use crate::runtime::browser::BrowserError;
use crate::runtime::protocol::RankingsCapture;
use crate::runtime::source::request::RankingsAction;
/// Build the navigation URL the source serves unchanged:
/// `/TrackAndField/rankings/list/{list_id}/{gender}/{event}?grades=N`.
///
/// The page depth travels in the API payload (`qParams.page`), never in this
/// landing URL. The source canonicalises a trailing slash and a `page` query
/// away, and a canonicalising navigation makes Chromium abort the original
/// document, which the navigation layer must not read as a transport failure.
pub(crate) fn build_ui_url(
    source_origin: &url::Url,
    action: &RankingsAction,
) -> Result<String, BrowserError> {
    let mut builder = source_origin
        .join(&format!(
            "/TrackAndField/rankings/list/{}/{}/{}",
            action.list_id, action.gender, action.event_short
        ))
        .map_err(|_| BrowserError::Protocol)?;
    if let Some(grade) = action.grade {
        builder
            .query_pairs_mut()
            .append_pair("grades", &grade.to_string());
    }
    Ok(builder.to_string())
}
#[derive(serde::Deserialize)]
struct RankingRequest<'a> {
    #[serde(rename = "qParams")]
    q_params: RankingQueryParams,
    #[serde(rename = "divListId")]
    div_list_id: u64,
    #[serde(rename = "eventShort")]
    event_short: &'a str,
    gender: &'a str,
}

#[derive(serde::Deserialize)]
struct RankingQueryParams {
    page: Option<u32>,
    grades: Vec<u64>,
}

#[derive(serde::Deserialize)]
struct RankingsEnvelope {
    #[serde(rename = "groupedRankings")]
    grouped_rankings: Vec<Vec<serde::de::IgnoredAny>>,
    #[serde(default)]
    settings: Option<EnvelopeSettings>,
    #[serde(rename = "defaultSettings", default)]
    default_settings: Option<EnvelopeSettings>,
}

/// The page-depth fields the source declares for a list response. Rows stay
/// `IgnoredAny`; only the declared depth is materialized.
#[derive(serde::Deserialize)]
struct EnvelopeSettings {
    depth: Option<u64>,
    page: Option<u64>,
}

impl RankingsEnvelope {
    fn row_count(&self) -> u64 {
        self.grouped_rankings
            .iter()
            .map(|group| {
                // A group longer than `u64::MAX` cannot be represented in the
                // observation; saturating keeps pagination from under-counting.
                u64::try_from(group.len()).map_or(u64::MAX, |count| count)
            })
            .sum()
    }

    /// The declared page depth: `settings.depth` when the request echoed its
    /// settings, otherwise the default settings echoed alongside.
    fn declared_page_depth(&self) -> Option<u64> {
        self.settings
            .as_ref()
            .and_then(|settings| settings.depth)
            .or_else(|| {
                self.default_settings
                    .as_ref()
                    .and_then(|settings| settings.depth)
            })
    }

    /// The page this body declares for itself: `settings.page` when the request
    /// echoed its settings, otherwise the default settings echoed alongside.
    fn declared_page(&self) -> Option<u64> {
        self.settings
            .as_ref()
            .and_then(|settings| settings.page)
            .or_else(|| {
                self.default_settings
                    .as_ref()
                    .and_then(|settings| settings.page)
            })
    }
}

/// Parse and validate the captured request body once, returning its validated page.
pub(crate) fn validate_request(
    captured: &CapturedRanking,
    action: &RankingsAction,
    allow_page_one: bool,
) -> Result<Option<u32>, BrowserError> {
    if action.capture == RankingsCapture::Navigation {
        return Ok(None);
    }
    let body = captured
        .request_body
        .as_deref()
        .ok_or(BrowserError::Protocol)?;
    let request: RankingRequest<'_> =
        serde_json::from_str(body).map_err(|_| BrowserError::Protocol)?;
    let Some(request_page) = request.q_params.page else {
        return Ok(None);
    };
    let page_matches =
        request_page == action.page || (allow_page_one && action.page > 1 && request_page == 1);
    if !page_matches {
        return Ok(None);
    }
    let grades_match = match action.grade {
        Some(grade) => {
            request.q_params.grades.len() == 1
                && request.q_params.grades.first() == Some(&u64::from(grade))
        }
        None => request.q_params.grades.is_empty(),
    };
    if !grades_match
        || request.div_list_id != action.list_id
        || request.event_short != action.event_short
        || request.gender != action.gender
    {
        return Ok(None);
    }
    Ok(Some(request_page))
}

/// The row count and declared page depth of a captured list response, used to
/// decide whether another page can exist. A body that is not a rankings
/// envelope is a protocol error for the caller to treat as terminal.
pub(crate) fn page_extent(body: &[u8]) -> Result<(u64, Option<u64>), BrowserError> {
    let envelope: RankingsEnvelope =
        serde_json::from_slice(body).map_err(|_| BrowserError::Protocol)?;
    Ok((envelope.row_count(), envelope.declared_page_depth()))
}

/// The page number the captured response body declares for itself. The source
/// answers a request past the listing's last page with the listing's first page,
/// which is how the pagination chain learns the list is exhausted.
pub(crate) fn declared_page(body: &[u8]) -> Option<u32> {
    let envelope: RankingsEnvelope = serde_json::from_slice(body).ok()?;
    let page = envelope.declared_page()?;
    u32::try_from(page).ok()
}
