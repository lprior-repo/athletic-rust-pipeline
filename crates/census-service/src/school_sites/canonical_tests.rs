use super::super::{run, SchoolSitesArgs};
use census_crawl::net::Fetcher;
use census_domain::model::{
    normalize_name, school_mailbox, school_mailbox_research, CanonicalSchool, CensusRun,
    ContactResearchOutcome, Evidence, GradYear, RunManifest, SchoolMailboxPurpose, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const URL: &str = "https://cacmustangs.org/about/contact/";
const NAME: &str = "Central Arkansas Christian Schools";
const CAPTURE: &[u8] = include_bytes!(
    "../../../census-crawl/src/coach_directories/generic/fixtures/cac-contact-extract.html"
);

fn setup(
    names: &[&str],
    year: SchoolYear,
) -> TestResult<(tempfile::TempDir, Store, Fetcher, SchoolSitesArgs)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Arkansas, NAME, normalize_name(NAME), None);
    school.aliases.push("CAC".to_owned());
    school.school_website = Some(URL.to_owned());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::SchoolDirectory {
                provider: "public_school_site".to_owned(),
                state: UsJurisdiction::Arkansas,
            },
            NAME,
        )
        .with_url(URL),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new("public_school_site", Some(URL.to_owned())),
        "2026-10-07T12:34:56Z",
    ));
    store.append_many(Table::Schools, &[school])?;
    store.bind_run(&RunManifest {
        store_identity: census_report::export::store_identity(&store)?,
        run: CensusRun::new(year, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Arkansas],
    })?;
    let input = dir.path().join("queue.jsonl");
    let rows = names
        .iter()
        .map(|name| serde_json::json!({"state":"AR","name":name,"website":URL}).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&input, rows)?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["cacmustangs.org".to_owned()],
    )?
    .with_offline(true);
    seed_cache(fetcher.cache_dir())?;
    let args = SchoolSitesArgs {
        input,
        out: Some(dir.path().join("out")),
        state: None,
        limit: None,
        sample: None,
        refresh: false,
        authorize_queue_hosts: false,
    };
    Ok((dir, store, fetcher, args))
}

fn seed_cache(dir: &std::path::Path) -> TestResult {
    let mut hasher = Sha256::new();
    hasher.update(b"GET\x1f");
    hasher.update(URL.as_bytes());
    hasher.update(b"\x1f");
    let hash = format!("{:x}", hasher.finalize());
    let key = hash.get(..32).ok_or("cache identity")?;
    let meta = serde_json::json!({ "url": URL, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(CAPTURE)), "bytes": CAPTURE.len(),
        "fetched_at": "2026-10-07T12:34:56Z", "content_type": "text/html" });
    std::fs::write(dir.join(format!("{key}.body")), CAPTURE)?;
    std::fs::write(
        dir.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta)?,
    )?;
    Ok(())
}

#[tokio::test]
async fn source_bound_alias_acquires_mailbox_without_minting_unmatched_queue_population(
) -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let (_dir, store, fetcher, args) = setup(&["CAC", "Unpublished Queue School"], year)?;
    let report = run(&fetcher, &args, &store).await?;
    check!(eq; (report.planned, report.crawled, report.failed, report.emails), (2, 1, 1, 1));
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    check!(eq; schools.iter().map(|school| school.name.as_str()).collect::<Vec<_>>(), vec![NAME]);
    let school = schools.first().ok_or("retained school")?;
    let claim = school_mailbox(school, SchoolMailboxPurpose::SchoolOffice, year)?
        .ok_or("published school office")?;
    check!(eq; claim.mailbox.as_str(), "cac@cacmustangs.org");
    check!(eq; claim.source.url.as_deref(), Some(URL));
    check!(eq; claim.source_sha256, format!("{:x}", Sha256::digest(CAPTURE)));
    check!(eq; claim.acquired_at.as_str(), "2026-10-07T12:34:56Z");
    check!(eq; report.failures.first().map(|failure| failure.school.as_str()), Some("Unpublished Queue School"));
    Ok(())
}

#[tokio::test]
async fn bound_run_year_leaves_prior_physical_capture_stale_instead_of_relabeling_it() -> TestResult
{
    let year = SchoolYear::new(2027).ok_or("year")?;
    let (_dir, store, fetcher, args) = setup(&[NAME], year)?;
    let report = run(&fetcher, &args, &store).await?;
    check!(eq; (report.planned, report.crawled, report.failed, report.emails), (1, 1, 1, 0));
    let school = store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .next()
        .ok_or("retained school")?;
    check!(eq; school_mailbox(&school, SchoolMailboxPurpose::SchoolOffice, year)?, None);
    check!(eq; school_mailbox_research(&school, SchoolMailboxPurpose::SchoolOffice, year), ContactResearchOutcome::Stale);
    check!(eq; school.mailbox_claims.first().map(|claim| claim.acquired_at.as_str()), Some("2026-10-07T12:34:56Z"));
    Ok(())
}

#[tokio::test]
async fn default_jurisdiction_cannot_relabel_an_explicit_foreign_queue_subject() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let (_dir, store, fetcher, mut args) = setup(&[NAME], year)?;
    std::fs::write(
        &args.input,
        serde_json::json!({"state":"Ontario","name":NAME,"website":URL}).to_string(),
    )?;
    args.state = Some("AR".to_owned());
    let report = run(&fetcher, &args, &store).await?;
    check!(eq; (report.planned, report.crawled, report.failed, report.requests), (0, 0, 1, 0));
    let school = store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .next()
        .ok_or("retained school")?;
    check!(eq; school_mailbox(&school, SchoolMailboxPurpose::SchoolOffice, year)?, None);
    check!(eq; school_mailbox_research(&school, SchoolMailboxPurpose::SchoolOffice, year), ContactResearchOutcome::Unattempted);
    Ok(())
}
