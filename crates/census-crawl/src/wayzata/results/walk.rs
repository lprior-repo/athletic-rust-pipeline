use super::parse::{parse_performances, Performance};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalPerformance, Evidence, Id, Mark, SourceIdentity, SourceNamespace, SourceRef,
};
use census_store::Table;

pub struct ResultsWalk {
    pub collected: usize,
}

impl ResultsWalk {
    pub fn new() -> Self {
        Self { collected: 0 }
    }

    pub async fn collect_for_slug(
        &mut self,
        ctx: &AdapterContext<'_>,
        slug: &str,
        meet_name: &str,
        meet_date: &str,
    ) -> CrawlResult<()> {
        let url = format!("https://www.wayzataresults.com/links/{slug}");
        let options = ctx.fetch_options();
        let fetched = match ctx.fetcher.get(&url, &options).await {
            Ok(f) => f,
            Err(error) => {
                return Ok(());
            }
        };
        let body = match validate(&fetched) {
            Ok(b) => b,
            Err(_) => {
                return Ok(());
            }
        };
        let performances = match parse_performances(body) {
            Ok(p) => p,
            Err(_) => {
                return Ok(());
            }
        };
        let mut batch = ctx.write_batch();
        for perf in &performances {
            if perf.athlete.is_empty() || perf.mark.is_empty() {
                continue;
            }
            match build_performance(&fetched, meet_name, meet_date, perf) {
                Ok(performance) => {
                    match batch.append_many(Table::Performances, &[performance]) {
                        Ok(_) => self.collected += 1,
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        }
        Ok(())
    }
}

fn validate(fetched: &crate::net::FetchOutcome) -> CrawlResult<&str> {
    if fetched.status != 200 {
        return Err(CrawlError::Schema {
            url: fetched.url.clone(),
            detail: "non-200 status".to_string(),
        });
    }
    Ok(std::str::from_utf8(&fetched.body).map_err(|_| CrawlError::Schema {
        url: fetched.url.clone(),
        detail: "invalid UTF-8".to_string(),
    })?)
}

fn build_performance(
    fetched: &crate::net::FetchOutcome,
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
    let meet_id = Id::mint("meet", &[meet_name, meet_date]);
    Ok(CanonicalPerformance {
        id: CanonicalPerformance::mint(&athlete_id, &meet_id, &event_id, meet_date, &source_key),
        athlete: athlete_id,
        team: team_id,
        event: event_id,
        meet: meet_id,
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