use std::collections::BTreeMap;

use anyhow::{Context, Result};
use census_crawl::wayzata::{self, ScheduleSport};
use census_crawl::AdapterReport;
use census_domain::model::{CanonicalMeet, CompetitionLevel};
use census_store::Table;
use serde::Serialize;

use super::{
    assert_rollup, common, context, golden, golden_case, seeded, stem_of, OBSERVED_ON, SEASON,
};

#[derive(Serialize)]
struct MeetsRun<'a> {
    report: &'a AdapterReport,
    meets: &'a [CanonicalMeet],
}

#[derive(Serialize)]
struct MeetRowFacts {
    date: String,
    name: String,
    location: String,
    slug: Option<String>,
    aria_label: Option<String>,
    venue_state: Option<String>,
    level: CompetitionLevel,
}

impl MeetRowFacts {
    fn of(row: &wayzata::MeetRow) -> Self {
        let wayzata::MeetRow {
            date,
            name,
            location,
            slug,
            aria_label,
        } = row;
        Self {
            date: date.clone(),
            name: name.clone(),
            location: location.clone(),
            slug: slug.clone(),
            aria_label: aria_label.clone(),
            venue_state: wayzata::venue_state(location).map(|state| state.code().to_string()),
            level: wayzata::level_of(name),
        }
    }
}

#[test]
fn wayzata_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("wayzata")?;
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let stem = stem_of(&file);
        let rows = wayzata::schedule_rows(&common::fixture("wayzata", &file)?, SEASON)
            .with_context(|| format!("reading {file}"))?;
        let facts = rows.iter().map(MeetRowFacts::of).collect::<Vec<_>>();
        let (name, digest) = golden_case(&format!("wayzata__{stem}"), &facts)?;
        cases.insert(name, digest);
    }
    assert_rollup("wayzata", cases, paths.len())
}

#[test]
fn wayzata_collect_from_a_seeded_cache_matches_golden() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir().context("temp dir")?;
            let (store, fetcher) = seeded(
                dir.path(),
                &[
                    (
                        &wayzata::schedule_url(ScheduleSport::Track, SEASON),
                        &common::fixture("wayzata", "track_2026_schedule.html")?,
                    ),
                    (
                        &wayzata::schedule_url(ScheduleSport::CrossCountry, SEASON),
                        &common::fixture("wayzata", "xc_2026_schedule.html")?,
                    ),
                ],
            )?;
            let options = wayzata::Options {
                years: vec![SEASON],
                limit: None,
                refresh: false,
                observed_on: Some(OBSERVED_ON.to_string()),
            };
            let report = wayzata::collect(&context(&fetcher, &store), &options).await?;

            let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
            anyhow::ensure!(
                report.requests == 0,
                "both schedules are answered from cache — left={:?} right={:?}",
                &report.requests,
                &0
            );
            anyhow::ensure!(
                report.from_cache == 2,
                "one cached page per sport — left={:?} right={:?}",
                &report.from_cache,
                &2
            );
            anyhow::ensure!(
                report.rows == 23,
                "13 track rows plus 10 cross-country rows: {report:?} — left={:?} right={:?}",
                &report.rows,
                &23
            );
            anyhow::ensure!(
                meets.len() >= 20,
                "most schedule rows mint a distinct meet; the store holds {}",
                meets.len()
            );
            golden::assert_golden(
                "wayzata__collect_2026_schedules",
                &MeetsRun {
                    report: &report,
                    meets: &meets,
                },
            )
        })
}
