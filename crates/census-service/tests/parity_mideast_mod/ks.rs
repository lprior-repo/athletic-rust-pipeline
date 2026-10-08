use anyhow::{ensure, Context, Result};
use census_crawl::{ks, CollectionDisposition};
use census_domain::model::{CanonicalCoach, CanonicalSchool};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use super::{common, context, seeded, OBSERVED_ON, SEEDED_AT};

#[test]
fn ks_collection_preserves_published_owners_and_reports_unmeasured_indexes() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let url = "https://kshsaa-api.kshsaa.org/directory/search/name/a/";
            let dir = tempfile::tempdir()?;
            let (store, fetcher) = seeded(
                dir.path(),
                &[(url, &common::fixture("ks", "kshsaa_directory_a.json")?)],
            )?;
            let options = ks::Options {
                limit: None,
                refresh: false,
                observed_on: OBSERVED_ON.into(),
                states: Vec::new(),
                school_names: Vec::new(),
            };
            let report = ks::collect(&context(&fetcher, &store)?, &options).await?;
            let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
            let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
            ensure!(
                report.requests == 0 && (report.rows, schools.len(), coaches.len()) == (5, 5, 5)
            );
            ensure!(
                report.disposition == CollectionDisposition::Partial
                    && report.unfinished
                        == vec!["https://kshsaa-api.kshsaa.org/directory/search/name/".to_string()]
            );
            for coach in &coaches {
                let owner = schools
                    .iter()
                    .find(|school| school.id == coach.school)
                    .context("published school owner")?;
                ensure!(owner.state == Some(UsJurisdiction::Kansas));
                ensure!(
                    coach
                        .evidence
                        .first()
                        .context("coach acquisition evidence")?
                        .observed_on
                        == SEEDED_AT
                );
                ensure!(coach
                    .evidence
                    .iter()
                    .all(|evidence| evidence.observed_on == SEEDED_AT));
            }
            let physical = store.stats()?.tables;
            let receipts = store.receipt_count()?;
            drop(store);
            let reopened = Store::open(dir.path().join("store"))?;
            let replay = ks::collect(&context(&fetcher, &reopened)?, &options).await?;
            ensure!(replay.rows == 0 && replay.requests == 0);
            ensure!(
                replay.disposition == report.disposition && replay.unfinished == report.unfinished
            );
            ensure!(reopened.scan::<CanonicalSchool>(Table::Schools)? == schools);
            ensure!(reopened.scan::<CanonicalCoach>(Table::Coaches)? == coaches);
            ensure!(reopened.stats()?.tables == physical && reopened.receipt_count()? == receipts);
            Ok(())
        })
}
