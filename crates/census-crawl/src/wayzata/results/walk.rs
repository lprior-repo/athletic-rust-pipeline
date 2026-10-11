use super::parse::{parse_performances, Performance};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{CanonicalPerformance, Evidence, Id, Mark, SourceRef};
use census_domain::model::tag::Meet;
use census_store::Table;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultsDisposition {
    Complete { collected: u64 },
    Empty,
    Partial { collected: u64, reason: String },
    Failed { url: String, status: u16, cause: String },
    Refused { url: String, cause: String },
}

pub struct ResultsWalk {
    pub collected: u64,
}

impl ResultsWalk {
    pub fn new() -> Self {
        Self { collected: 0 }
    }

    pub async fn collect_for_slug(
        &mut self,
        ctx: &AdapterContext<'_>,
        slug: &str,
        meet_id: &Id<Meet>,
        meet_name: &str,
        meet_date: &str,
    ) -> CrawlResult<ResultsDisposition> {
        let url = format!("https://www.wayzataresults.com/links/{slug}");
        let options = ctx.fetch_options();
        let fetched = match ctx.fetcher.get(&url, &options).await {
            Ok(fetched) => fetched,
            Err(error) => {
                return Ok(ResultsDisposition::Failed {
                    url: url.clone(),
                    status: 0,
                    cause: format!("network error: {error}"),
                });
            }
        };
        let body = match validate(&fetched) {
            Ok(body) => body,
            Err(error) => {
                return Ok(ResultsDisposition::Failed {
                    url: fetched.url.clone(),
                    status: fetched.status,
                    cause: format!("validation error: {error}"),
                });
            }
        };
        let performances = match parse_performances(body) {
            Ok(performances) => performances,
            Err(error) => {
                return Ok(ResultsDisposition::Failed {
                    url: fetched.url.clone(),
                    status: fetched.status,
                    cause: format!("parse error: {error}"),
                });
            }
        };
        let mut batch = ctx.write_batch();
        let mut appended = 0u64;
        for perf in &performances {
            if perf.athlete.is_empty() || perf.mark.is_empty() {
                continue;
            }
            if let Ok(performance) = build_performance(&fetched, meet_id, meet_name, meet_date, perf) {
                if batch
                    .append_many(Table::Performances, &[performance])
                    .is_ok()
                {
                    appended = super::super::budget::add(appended, 1)?;
                }
            }
        }
        if appended > 0 {
            let payload = format!("wayzata_results:{meet_name}:{meet_date}:{}", fetched.content_digest);
            let operation = "wayzata_results_projection";
            let digest = census_domain::model::serialized_digest(&payload)
                .map_err(|source| CrawlError::Canonical {
                    table: operation.to_string(),
                    source,
                })?;
            batch.journal_done("wayzata_results_projection", &digest, &digest)?;
            let wrote = batch.commit_once(operation, &digest)?.written();
        }
        self.collected = super::super::budget::add(self.collected, appended)?;
        if appended > 0 {
            Ok(ResultsDisposition::Complete { collected: appended })
        } else {
            Ok(ResultsDisposition::Empty)
        }
    }
}

fn validate(fetched: &crate::net::FetchOutcome) -> CrawlResult<&str> {
    if fetched.status != 200 {
        return Err(CrawlError::Schema {
            url: fetched.url.clone(),
            detail: "non-200 status".to_string(),
        });
    }
    std::str::from_utf8(&fetched.body).map_err(|_| CrawlError::Schema {
        url: fetched.url.clone(),
        detail: "invalid UTF-8".to_string(),
    })
}

fn build_performance(
    fetched: &crate::net::FetchOutcome,
    meet_id: &Id<Meet>,
    meet_name: &str,
    meet_date: &str,
    perf: &Performance,
) -> CrawlResult<CanonicalPerformance> {
    let mark_value = parse_mark(&perf.mark);
    let source_ref = SourceRef::new("wayzata", Some(fetched.url.clone()));
    let mut evidence = Evidence::parsed(source_ref, &fetched.fetched_at);
    evidence.note = Some(format!("meet: {meet_name}"));
    let source_key = format!("wayzata|{meet_name}|{meet_date}|{}", perf.mark);
    let athlete_id = Id::mint("ath", &[&perf.athlete]);
    let team_id = Id::mint("team", &[&perf.team]);
    let event_id = Id::mint("evt", &[&perf.event]);
    Ok(CanonicalPerformance {
        id: CanonicalPerformance::mint(&athlete_id, &meet_id, &event_id, meet_date, &source_key),
        athlete: athlete_id,
        team: team_id,
        event: event_id,
        meet: meet_id.clone(),
        date: meet_date.to_string(),
        mark: mark_value,
        wind_mps: None,
        place: None,
        heat: None,
        round: Some("final".to_string()),
        timing: None,
        observed_grade: None,
        evidence: vec![evidence],
        source_key,
        source_athlete: None,
        retained_conflicts: Vec::new(),
    })
}

fn parse_mark(mark: &str) -> Mark {
    Mark::Raw(mark.to_string())
}
