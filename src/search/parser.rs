//! The live `POST https://www.athletic.net/Search.aspx/runSearch` response contract.
//!
//! The endpoint serves its envelope as JSON —
//! `{"d":{"__type":"Search+SearchResults","count":…,"pager":…,"results":…,"runTime":…}}` — where
//! `d.results` holds the result markup the parser keys on, `d.pager` the next-offset pager, and
//! `d.count` the query-wide row total that `progress` reconciles the walk against. Rows are keyed on
//! the `<tr>` element and on the `/athlete/<id>/<sport>` href, never on the decorative markup around
//! them (`img` flags, `span.sportIcon`, team links, PR tooltips, css classes, column counts). Rows
//! this query cannot use are counted, not rejected: see `SearchPage::skipped`. Responses that are
//! not this page at all are rejected by name: see `PageRejection`.
use super::SearchQuery;
use crate::domain::{
    evidence::{EvidenceRef, Sport},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

mod markup;
mod rejection;

pub use rejection::PageRejection;

const MAX_PAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchIssue {
    pub code: String,
    pub message: String,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCandidate {
    pub athlete_id: AthleteId,
    pub profile_url: ProfileUrl,
    pub display_name: String,
    pub snippet: String,
    pub sport: Sport,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchPage {
    pub query: SearchQuery,
    pub start: u32,
    pub count: u32,
    pub requested_count: u32,
    pub candidates: Vec<SearchCandidate>,
    pub issues: Vec<SearchIssue>,
    /// Result rows the endpoint delivered that this query cannot use: sibling-sport rows and the
    /// site's placeholder rows for athletes it will not identify. The site advertises rows, so they
    /// are part of `count`; they never become candidates and never become page issues. Absent in
    /// records written before this field existed.
    #[serde(default)]
    pub skipped: u32,
    pub next_offset: Option<u32>,
    pub pager: String,
    pub run_time: Option<String>,
}

#[derive(Deserialize)]
struct Envelope {
    d: Payload,
}

#[derive(Deserialize)]
struct Payload {
    results: String,
    count: Count,
    pager: String,
    #[serde(rename = "runTime")]
    run_time: Option<RunTime>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Count {
    Number(u32),
    Text(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RunTime {
    Number(serde_json::Number),
    Text(String),
}

/// Parse and validate the search response envelope.
fn envelope(bytes: &[u8]) -> anyhow::Result<(Payload, u32)> {
    let Envelope { d: payload } = serde_json::from_slice(bytes)
        .map_err(|error| anyhow::anyhow!("invalid search envelope: {error}"))?;
    let count = match &payload.count {
        Count::Number(value) => *value,
        Count::Text(value) => value
            .trim()
            .parse()
            .map_err(|error| anyhow::anyhow!("invalid search count: {error}"))?,
    };
    Ok((payload, count))
}

pub fn parse_page(
    query: &SearchQuery,
    start: u32,
    digest: EvidenceDigest,
    bytes: &[u8],
) -> Result<SearchPage> {
    if bytes.len() > MAX_PAGE_BYTES {
        return Err(PageRejection::Malformed {
            detail: format!("body exceeds {MAX_PAGE_BYTES} bytes"),
        }
        .into());
    }
    let (payload, count) = envelope(bytes).map_err(|error| PageRejection::NotJson {
        detail: format!("invalid search envelope: {error}"),
    })?;
    let markup::Rows {
        candidates,
        issues,
        seen,
        skipped,
    } = markup::rows(query, start, &digest, &payload.results).map_err(|error| {
        PageRejection::Malformed {
            detail: format!("{error:#}"),
        }
    })?;
    if seen == 0 && count > 0 {
        return Err(PageRejection::RowsAbsent {
            detail: format!(
                "envelope advertises {count} result(s) and /d/results carries no result row"
            ),
        }
        .into());
    }
    let requested_count =
        u32::try_from(candidates.len()).context("requested result count overflowed")?;
    let run_time = payload.run_time.map(|value| match value {
        RunTime::Number(value) => value.to_string(),
        RunTime::Text(value) => value,
    });
    Ok(SearchPage {
        query: query.clone(),
        start,
        count,
        requested_count,
        candidates,
        issues,
        skipped,
        next_offset: markup::next_offset(&payload.pager, start).map_err(|error| {
            PageRejection::Malformed {
                detail: format!("{error:#}"),
            }
        })?,
        pager: payload.pager,
        run_time,
    })
}

impl SearchCandidate {
    #[must_use]
    pub fn id(&self) -> AthleteId {
        self.athlete_id
    }
    #[must_use]
    pub fn url(&self) -> &ProfileUrl {
        &self.profile_url
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.display_name
    }
}

impl SearchPage {
    #[must_use]
    pub fn advertised_count(&self) -> u32 {
        self.count
    }
    #[must_use]
    pub fn results(&self) -> &[SearchCandidate] {
        &self.candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(sport: Sport, other_id: u64) -> Result<SearchPage> {
        let query = SearchQuery::new("Synthetic Runner", sport, 0)?;
        let digest = EvidenceDigest::parse(&"a".repeat(64))?;
        let body = serde_json::json!({"d":{"count":1,"pager":"","results":format!(
            "<tr><td><a href='/athlete/7/track-and-field'>Synthetic Runner</a>\
             <a href='/athlete/{other_id}/cross-country'><span class='xc-icon'></span></a>\
             Fictional High</td></tr>")}});
        parse_page(&query, 0, digest, &serde_json::to_vec(&body)?)
    }

    #[test]
    fn one_identity_with_two_sport_links_is_one_candidate_in_either_search() -> Result<()> {
        for (sport, suffix) in [
            (Sport::TrackField, "track-and-field"),
            (Sport::CrossCountry, "cross-country"),
        ] {
            let page = parse(sport, 7)?;
            anyhow::ensure!(
                page.requested_count == 1,
                "left={:?} right={:?}",
                &page.requested_count,
                &1
            );
            anyhow::ensure!(page.issues.is_empty());
            anyhow::ensure!(
                page.candidates[0].athlete_id.get() == 7,
                "left={:?} right={:?}",
                &page.candidates[0].athlete_id.get(),
                &7
            );
            anyhow::ensure!(
                page.candidates[0].display_name == "Synthetic Runner",
                "left={:?} right={:?}",
                &page.candidates[0].display_name,
                &"Synthetic Runner"
            );
            anyhow::ensure!(page.candidates[0].profile_url.as_str().ends_with(suffix));
        }
        Ok(())
    }

    #[test]
    fn a_row_linking_different_athletes_cannot_become_a_candidate() -> Result<()> {
        let page = parse(Sport::TrackField, 8)?;
        anyhow::ensure!(page.candidates.is_empty());
        anyhow::ensure!(
            page.issues.len() == 1,
            "left={:?} right={:?}",
            &page.issues.len(),
            &1
        );
        anyhow::ensure!(
            page.issues[0].code == "invalid_athlete_row",
            "left={:?} right={:?}",
            &page.issues[0].code,
            &"invalid_athlete_row"
        );
        Ok(())
    }

    #[test]
    fn missing_athlete_id_cannot_become_a_complete_empty_search() -> Result<()> {
        let query = SearchQuery::new("Synthetic Runner", Sport::TrackField, 0)?;
        let digest = EvidenceDigest::parse(&"a".repeat(64))?;
        let body = serde_json::json!({"d":{"count":0,"pager":"","results":
            "<tr><td><a href='/athlete//track-and-field'>Synthetic Runner</a></td></tr>"}});
        let page = parse_page(&query, 0, digest, &serde_json::to_vec(&body)?)?;
        // The site's placeholder row is a row this query cannot use, so it is counted as a skip and
        // the page still does not describe an empty search.
        anyhow::ensure!(page.candidates.is_empty());
        anyhow::ensure!(page.issues.is_empty());
        anyhow::ensure!(page.skipped == 1, "left={:?} right={:?}", &page.skipped, &1);
        let mut progress = crate::search::SearchProgress::new(query);
        anyhow::ensure!(progress.consume(page).is_err());
        anyhow::ensure!(!progress.complete());
        Ok(())
    }

    #[test]
    fn row_text_grown_past_the_bound_cannot_become_a_candidate() -> Result<()> {
        let query = SearchQuery::new("Synthetic Runner", Sport::TrackField, 0)?;
        let digest = EvidenceDigest::parse(&"a".repeat(64))?;
        // Two 5 KiB text nodes stay under the 8 KiB per-node bound but overflow the row text bound.
        let head = "x".repeat(5_000);
        let tail = "y".repeat(5_000);
        let body = serde_json::json!({"d":{"count":1,"pager":"","results":format!(
            "<tr><td><a href='/athlete/7/track-and-field'>Synthetic Runner</a>\
             {head}<span>{tail}</span></td></tr>")}});
        let page = parse_page(&query, 0, digest, &serde_json::to_vec(&body)?)?;
        anyhow::ensure!(page.candidates.is_empty());
        anyhow::ensure!(
            page.issues.len() == 1,
            "left={:?} right={:?}",
            &page.issues.len(),
            &1
        );
        anyhow::ensure!(page.issues.iter().any(|issue| {
            issue.code == "invalid_athlete_row" && issue.message == "search row text exceeds bound"
        }));
        Ok(())
    }
}
