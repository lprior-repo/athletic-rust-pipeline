use super::SearchQuery;
use crate::domain::{
    evidence::{EvidenceRef, Sport},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

mod markup;
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

pub fn parse_page(
    query: &SearchQuery,
    start: u32,
    digest: EvidenceDigest,
    bytes: &[u8],
) -> Result<SearchPage> {
    if bytes.len() > MAX_PAGE_BYTES {
        bail!("search page exceeds {MAX_PAGE_BYTES} bytes");
    }
    let Envelope { d: payload } =
        serde_json::from_slice(bytes).context("invalid search envelope")?;
    let count = match payload.count {
        Count::Number(value) => value,
        Count::Text(value) => value.trim().parse().context("invalid search count")?,
    };
    let (candidates, issues) = markup::rows(query, start, &digest, &payload.results)?;
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
        next_offset: markup::next_offset(&payload.pager, start)?,
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
            assert_eq!(page.requested_count, 1);
            assert!(page.issues.is_empty());
            assert_eq!(page.candidates[0].athlete_id.get(), 7);
            assert_eq!(page.candidates[0].display_name, "Synthetic Runner");
            assert!(page.candidates[0].profile_url.as_str().ends_with(suffix));
        }
        Ok(())
    }

    #[test]
    fn a_row_linking_different_athletes_cannot_become_a_candidate() -> Result<()> {
        let page = parse(Sport::TrackField, 8)?;
        assert!(page.candidates.is_empty());
        assert_eq!(page.issues.len(), 1);
        assert_eq!(page.issues[0].code, "invalid_athlete_row");
        Ok(())
    }

    #[test]
    fn missing_athlete_id_cannot_become_a_complete_empty_search() -> Result<()> {
        let query = SearchQuery::new("Synthetic Runner", Sport::TrackField, 0)?;
        let digest = EvidenceDigest::parse(&"a".repeat(64))?;
        let body = serde_json::json!({"d":{"count":0,"pager":"","results":
            "<tr><td><a href='/athlete//track-and-field'>Synthetic Runner</a></td></tr>"}});
        let page = parse_page(&query, 0, digest.clone(), &serde_json::to_vec(&body)?)?;
        assert!(page.candidates.is_empty());
        assert!(page.issues.iter().any(|issue| {
            issue.code == "invalid_athlete_row" && issue.evidence.document == digest
        }));
        let mut progress = crate::search::SearchProgress::new(query);
        assert!(progress.consume(page).is_err());
        assert!(!progress.complete());
        Ok(())
    }
}
