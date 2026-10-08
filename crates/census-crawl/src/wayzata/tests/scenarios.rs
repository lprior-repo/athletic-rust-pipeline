use super::{seed_cache, TestResult, OBSERVED_ON};
use crate::net::Fetcher;
use crate::wayzata::{collect, schedule_url, Options, ScheduleSport};
use crate::{AdapterContext, AdapterReport};
use census_domain::model::{CanonicalMeet, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::HashMap;

mod dates;
mod frontier;
mod geography;

const EMPTY: &str = "<table class=\"schedule\"></table>";
const FETCHED: &str = "2026-09-20T14:39:00Z";

struct Harness {
    _root: tempfile::TempDir,
    store: Store,
    fetcher: Fetcher,
}

impl Harness {
    fn new(track: &str, xc: Option<&str>) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(
            store.http_cache_dir(),
            None,
            std::time::Duration::ZERO,
            HashMap::new(),
            Vec::new(),
        )?
        .with_offline(true);
        let harness = Self {
            _root: root,
            store,
            fetcher,
        };
        harness.seed(ScheduleSport::Track, 2026, track)?;
        if let Some(body) = xc {
            harness.seed(ScheduleSport::CrossCountry, 2026, body)?;
        }
        Ok(harness)
    }

    fn seed(&self, sport: ScheduleSport, year: i16, body: &str) -> TestResult {
        seed_cache(
            &self.store.http_cache_dir(),
            &schedule_url(sport, year),
            body,
        )
    }

    async fn run(
        &self,
        years: &[i16],
        scope: &[UsJurisdiction],
        limit: Option<usize>,
        as_of: &str,
    ) -> TestResult<AdapterReport> {
        let ctx = AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2028).ok_or("school year")?,
            observed_on: OBSERVED_ON.to_string(),
            recording: None,
            performance_as_of: chrono::NaiveDate::parse_from_str(as_of, "%Y-%m-%d")?,
        };
        let options = Options {
            years: years.to_vec(),
            jurisdictions: scope.to_vec(),
            limit,
            refresh: false,
            observed_on: Some("2030-10-01T00:00:00Z".to_string()),
        };
        Ok(collect(&ctx, &options).await?)
    }

    fn meets(&self) -> TestResult<Vec<CanonicalMeet>> {
        let mut meets: Vec<CanonicalMeet> = self.store.scan(Table::Meets)?;
        meets.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(meets)
    }

    fn raw(&self) -> TestResult<Vec<serde_json::Value>> {
        Ok(self
            .store
            .journal_payloads(super::super::receipt::RAW_PHASE)?)
    }
}

fn schedule(month: &str, rows: &[(&str, &str, &str)]) -> String {
    let events: String = rows.iter().map(|(date, name, venue)| format!(
        "<tr class=\"event-row\"><td class=\"date\">{date}</td><td class=\"awayteam\">{name}</td><td class=\"hometeam\">{venue}</td></tr>"
    )).collect();
    format!(
        "<table class=\"schedule\"><tr class=\"month-title\"><td>{month}</td></tr>{events}</table>"
    )
}

fn row_locator(ordinal: usize) -> String {
    format!("{}#row={ordinal}", schedule_url(ScheduleSport::Track, 2026))
}

fn observed_meet_names(harness: &Harness) -> TestResult<Vec<String>> {
    let mut names = Vec::new();
    harness
        .store
        .snapshot()
        .for_each_observation(Table::Meets, |meet: CanonicalMeet| {
            names.push(meet.name);
            Ok(())
        })?;
    names.sort();
    Ok(names)
}
