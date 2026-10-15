use anyhow::{ensure, Context, Result};
use census_crawl::{milesplit, net::Fetcher, AdapterContext};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalPerformance, CanonicalSchool, SchoolYear,
    SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

#[path = "publication/readback.rs"]
mod readback;
pub use readback::assert_publication;

pub async fn project_into_publication_store(store: &Store, fetcher: &Fetcher) -> Result<()> {
    seed_directory_owners(store)?;
    let options = crate::milesplit_fixtures::replay::seed_captures(
        fetcher,
        crate::milesplit_fixtures::replay::OWNED,
    )?;
    let context = AdapterContext {
        capture: census_crawl::CaptureContext::LegacyMetadata,
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).context("fixture season")?,
        observed_on: "2026-09-20".into(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-09-20", "%Y-%m-%d")?,
        recording: None,
    };
    let report = milesplit::collect_result_sets(&context, &options).await?;
    ensure!(report.requests == 0 && report.from_cache == 3 && report.rows == 3);
    ensure!(report.errors == 0, "owned projection failed: {report:?}");
    assert_projected(store)?;
    let before = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.journal_payloads(milesplit::RESULT_SET_PHASE)?;
    let applied: Vec<_> = receipts
        .iter()
        .filter(|row| row["disposition"] == "projection_applied")
        .collect();
    ensure!(!applied.is_empty(), "owned projection receipt absent");
    for receipt in applied {
        ensure!(receipt["source_completeness"] == "unknown");
        ensure!(receipt["ownership_complete"] == false);
        ensure!(receipt["canonical_identity_accepted"] == false);
        ensure!(receipt["lifetime_pr_claimed"] == false);
        ensure!(receipt["census_sealed"] == false);
    }
    let repeated = milesplit::collect_result_sets(&context, &options).await?;
    ensure!(repeated.requests == 0);
    ensure!(store.snapshot().tables_digest(&Table::ALL)? == before);
    ensure!(store.journal_payloads(milesplit::RESULT_SET_PHASE)? == receipts);
    Ok(())
}

fn seed_directory_owners(store: &Store) -> Result<()> {
    let body = crate::common::fixture("milesplit", "al_teams_index.html")?;
    let directory = milesplit::parse_team_index(&body)?;
    for id in ["38332", "4912"] {
        let matches: Vec<_> = directory
            .teams
            .iter()
            .filter(|team| team.id == id)
            .collect();
        ensure!(matches.len() == 1, "directory owner {id} is not unique");
        let team = matches.first().context("directory school")?;
        let (mut school, _) = CanonicalSchool::new(
            UsJurisdiction::Alabama,
            &team.name,
            normalize_name(&team.name),
            None,
        );
        school.source_identities.push(
            SourceIdentity::new(SourceNamespace::MilesplitSchool, id).with_url(team.url.clone()),
        );
        store.append(Table::Schools, &school)?;
    }
    Ok(())
}

fn assert_projected(store: &Store) -> Result<()> {
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    for (key, owner, year, name) in [
        ("201782263", "14222592", 2027, "Adelyn Spann"),
        ("201782277", "14222592", 2027, "Adelyn Spann"),
        ("201782806", "11357806", 2026, "Payton Ousley"),
    ] {
        let rows: Vec<_> = performances
            .iter()
            .filter(|row| row.source_key == format!("milesplit_result:{key}"))
            .collect();
        ensure!(rows.len() == 1, "owned result {key} missing or duplicated");
        let result = rows.first().context("owned result")?;
        let athlete = athletes
            .iter()
            .find(|row| row.id == result.athlete)
            .context("owned athlete")?;
        ensure!(athlete.canonical_name == name && athlete.grad_year.get() == year);
        ensure!(
            athlete
                .identities()
                .any(|id| id.namespace == SourceNamespace::MilesplitAthlete
                    && id.id.as_str() == owner)
        );
        ensure!(result.date == "2026-03-27");
        ensure!(result
            .evidence
            .iter()
            .any(|fact| fact.observed_on == crate::milesplit_fixtures::replay::ACQUIRED_AT));
    }
    Ok(())
}
