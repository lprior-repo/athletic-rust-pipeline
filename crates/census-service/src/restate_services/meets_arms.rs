mod walks;

use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};

use crate::census::{self, MeetCensus, MeetSourceRows};
use census_crawl::net::Fetcher;
use census_crawl::{Recorded, Recording};
use census_store::Store;
use serde::{Deserialize, Serialize};

use super::jobs::assert_some_stage_arms;
use walks::{walk_index, Walk};

pub(super) const MEETS_ARMS: &[(&str, MeetsArm)] = &[
    ("wiaa_results", MeetsArm::WiaaResults),
    ("wayzata", MeetsArm::Wayzata),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MeetsStageOutcome {
    pub census: MeetCensus,
    pub recorded: Vec<RecordedSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RecordedSource {
    pub slug: String,
    pub recorded: Recorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeetsArm {
    WiaaResults,
    Wayzata,
}

pub(super) fn arm_for(slug: &str) -> Option<MeetsArm> {
    MEETS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

pub(super) async fn meets_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    year: u16,
    refresh: bool,
    at: String,
    sweepable: Vec<String>,
) -> Result<Json<MeetsStageOutcome>, HandlerError> {
    let season = season_of(year)?;
    let (mut census, index_recorded) =
        walk_index(&store, &fetcher, jurisdiction, year, refresh, &at).await?;
    let mut recorded = Vec::new();
    if let Some(entry) = index_recorded {
        recorded.push(entry);
    }
    let walk = Walk::new(&store, &fetcher, jurisdiction, season, refresh, &at);
    let mut sources = vec![MeetSourceRows {
        slug: census::SOURCE.to_string(),
        rows: census.rows,
    }];
    for slug in &sweepable {
        let Some(arm) = arm_for(slug) else {
            assert_some_stage_arms(slug)?;
            continue;
        };
        let recording = Recording::new();
        let rows = walk.armed(arm, &recording).await?;
        take_recorded(slug, recording.drain(), &mut recorded);
        sources.push(MeetSourceRows {
            slug: slug.clone(),
            rows,
        });
        census.rows = census.rows.saturating_add(rows);
    }
    census.sources = sources;
    Ok(Json(MeetsStageOutcome { census, recorded }))
}
fn take_recorded(slug: &str, walked: Recorded, recorded: &mut Vec<RecordedSource>) {
    if walked.is_empty() {
        return;
    }
    recorded.push(RecordedSource {
        slug: slug.to_string(),
        recorded: walked,
    });
}

pub(super) fn season_of(year: u16) -> Result<SchoolYear, HandlerError> {
    let start_year = i16::try_from(year).map_err(|_| not_a_season(year))?;
    SchoolYear::new(start_year).ok_or_else(|| not_a_season(year))
}

pub(super) fn not_a_season(year: u16) -> HandlerError {
    TerminalError::new(format!("season year {year} is not a school year")).into()
}
