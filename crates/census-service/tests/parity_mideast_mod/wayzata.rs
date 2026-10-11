use anyhow::{Context, Result};
use census_crawl::wayzata::{self, ScheduleSport};
use census_crawl::CollectionDisposition;
use census_domain::model::{CanonicalMeet, CanonicalPerformance};
use census_store::Table;

use super::{common, context, seeded, OBSERVED_ON, SEASON};

#[test]
fn wayzata_schedule_retains_unknown_venue_rows_without_inventing_canonical_geography() -> Result<()>
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir().context("temp dir")?;
            let track_url = wayzata::schedule_url(ScheduleSport::Track, SEASON);
            let xc_url = wayzata::schedule_url(ScheduleSport::CrossCountry, SEASON);
            let track = common::fixture("wayzata", "track_2026_schedule.html")?;
            let xc = common::fixture("wayzata", "xc_2026_schedule.html")?;
            let (store, fetcher) = seeded(dir.path(), &[(&track_url, &track), (&xc_url, &xc)])?;
            let options = wayzata::Options {
                years: vec![SEASON],
                jurisdictions: Vec::new(),
                limit: None,
                refresh: false,
                observed_on: Some(OBSERVED_ON.to_string()),
            };
            let report = wayzata::collect(&context(&fetcher, &store)?, &options).await?;
            anyhow::ensure!(
                report.disposition == CollectionDisposition::Partial,
                "unknown geography must remain owed: {report:?}"
            );
            anyhow::ensure!(
                report.unfinished.contains(&format!("{xc_url}#row=3")),
                "Bassett Creek Park needs its exact row locator: {report:?}"
            );
            let raw = store.journal_payloads("wayzata_schedule_capture_v4")?;
            anyhow::ensure!(
                raw.iter()
                    .any(|row| row["name"] == "Ron Kretsch Invitational"
                        && row["venue"] == "Bassett Creek Park"
                        && row["date"] == "2026-08-29"
                        && row["capture"]["url"] == xc_url
                        && row["capture"]["fetched_at"] == super::SEEDED_AT),
                "unresolved published venue metadata must survive projection"
            );
            let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
            anyhow::ensure!(
                !meets
                    .iter()
                    .any(|meet| meet.name == "Ron Kretsch Invitational"),
                "unknown venue must not mint a canonical meet"
            );
            anyhow::ensure!(
                meets
                    .iter()
                    .any(|meet| meet.name == "USATF Minnesota All-Comers Meet #3"
                        && meet.date == "2026-01-04"
                        && meet.state == Some(census_domain::UsJurisdiction::Minnesota)),
                "known published campus must still produce its canonical calendar meet"
            );
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            anyhow::ensure!(
                performances.is_empty(),
                "schedule links labeled Results do not constitute performances"
            );
            Ok(())
        })
}
