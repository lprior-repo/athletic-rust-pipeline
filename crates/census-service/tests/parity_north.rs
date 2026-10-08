#[path = "common/capture_cache.rs"]
mod capture_cache;
mod common;

use anyhow::{ensure, Context, Result};
use census_crawl::mshsl::{self, COACH_API_PREFIX, TEAMS_VIEW_URL};
use census_crawl::net::Fetcher;
use census_crawl::{plain_names, AdapterContext};
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender, SchoolYear, Sport};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::Duration;

const CAPTURED_AT: &str = "2026-09-20T14:39:00Z";

fn seed_cache(cache: &Path, url: &str, body: &str) -> Result<()> {
    capture_cache::seed(cache, url, body.as_bytes(), CAPTURED_AT, &[])
}

fn fetcher(cache: &Path) -> Result<Fetcher> {
    Ok(Fetcher::new(cache, None, Duration::ZERO, HashMap::new(), Vec::new())?.with_offline(true))
}

fn context<'a>(fetcher: &'a Fetcher, store: &'a Store) -> Result<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-09-20", "%Y-%m-%d")?,
        recording: None,
    })
}

#[test]
fn mshsl_collection_retains_published_coaches_and_physical_capture_evidence() -> Result<()> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let cache = dir.path().join("http");
        let listing = common::fixture("mshsl", "schools_listing_first_page.html")?;
        let detail_body = common::fixture("mshsl", "school_detail_aitkin-high-school.html")?;
        let teams_body = common::fixture("mshsl", "team_nodes_aitkin.json")?;
        let detail = mshsl::parse_school_detail(&detail_body);
        let rows = mshsl::parse_school_list(&listing);
        let row = rows.iter().find(|row| row.slug == "aitkin-high-school").context("Aitkin listing")?;
        let key = mshsl::provider_key(row, &detail);
        let teams_url = format!("{TEAMS_VIEW_URL}?views-argument%5B%5D={key}&fields%5Bnode--participant%5D=title,path,drupal_internal__nid");
        seed_cache(&cache, &mshsl::listing_page_url(0), &listing)?;
        seed_cache(&cache, &mshsl::school_page_url("aitkin-high-school"), &detail_body)?;
        seed_cache(&cache, &teams_url, &teams_body)?;
        for node in mshsl::select_team_nodes(&mshsl::parse_team_nodes(&teams_body)) {
            let file = if node.alias.contains("track-and-field-boys") {
                "coach_records_aitkin_track-and-field-boys.json"
            } else if node.alias.contains("track-and-field-girls") {
                "coach_records_aitkin_track-and-field-girls.json"
            } else { continue; };
            seed_cache(&cache, &format!("{COACH_API_PREFIX}{}", node.nid), &common::fixture("mshsl", file)?)?;
        }
        let root = dir.path().join("store");
        let store = Store::open(&root)?;
        let fetcher = fetcher(&cache)?;
        let options = mshsl::Options { limit: Some(1), school_names: vec!["Aitkin High School".into()],
            ..mshsl::Options::default() };
        let report = mshsl::collect(&context(&fetcher, &store)?, &options).await?;
        ensure!(report.requests == 0 && report.errors == 0, "{report:?}");
        let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
        ensure!(schools.len() == 1 && schools.first().is_some_and(|school| school.name == "Aitkin High School"));
        let school = schools.first().context("Aitkin canonical school")?;
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        let people = [
            ("Adam Carlson", Gender::Boys, Some(Sport::OutdoorTrack), CoachRole::HeadCoach, "acarlson@isd1.org"),
            ("Ava Carlson", Gender::Girls, Some(Sport::OutdoorTrack), CoachRole::HeadCoach, "avacarlson@isd1.org"),
            ("Jim Henrickson", Gender::Mixed, None, CoachRole::AthleticDirector, "jhenrickson@isd1.org"),
            ("Alan Hills", Gender::Mixed, None, CoachRole::AthleticDirector, "ahills@isd1.org"),
        ];
        ensure!(coaches.len() == people.len());
        for (name, gender, sport, role, mailbox) in people {
            let coach = coaches.iter().find(|coach| coach.name == name && coach.gender == gender).context("published coach")?;
            ensure!(coach.school == school.id && coach.sport == sport && coach.role == role);
            ensure!(coach.professional_email.as_deref() == Some(mailbox));
            ensure!(coach.evidence.first().context("coach acquisition evidence")?.observed_on == CAPTURED_AT);
            ensure!(coach.evidence.iter().all(|evidence| evidence.observed_on == CAPTURED_AT));
        }
        let physical = store.stats()?.tables;
        drop(store);
        let reopened = Store::open(&root)?;
        mshsl::collect(&context(&fetcher, &reopened)?, &options).await?;
        ensure!(reopened.scan::<CanonicalSchool>(Table::Schools)? == schools);
        ensure!(reopened.scan::<CanonicalCoach>(Table::Coaches)? == coaches);
        ensure!(reopened.stats()?.tables == physical);
        Ok(())
    })
}

#[test]
fn controlled_plain_names_inventory_is_offline_and_retains_published_school_owners() -> Result<()> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let cache = dir.path().join("http");
        let historical_index = common::fixture("plain_names", "nd_schools_index.html")?;
        let members = plain_names::parse_nd_school_refs(&historical_index)?;
        let sheyenne = members.iter().find(|member| member.id == "1045").context("Sheyenne member")?;
        let index = format!("<html data-fixture=\"synthetic-single-school-index\"><body><a href=\"{}\">West Fargo Sheyenne</a></body></html>", sheyenne.url());
        ensure!(plain_names::parse_nd_school_refs(&index)? == vec![sheyenne.clone()]);
        let form = "<html data-fixture=\"synthetic-single-school-form\"><select><option>Adams Central</option></select></html>";
        ensure!(plain_names::parse_nsaa_school_names(form)? == vec!["Adams Central".to_string()]);
        seed_cache(&cache, plain_names::ND_SCHOOLS_URL, &index)?;
        seed_cache(&cache, &sheyenne.url(), &common::fixture("plain_names", "nd_school_page.html")?)?;
        seed_cache(&cache, plain_names::NSAA_FORM_URL, form)?;
        seed_cache(&cache, &plain_names::nsaa_school_url("Adams Central"), &common::fixture("plain_names", "nsaa_school_get_adams_central.html")?)?;
        let root = dir.path().join("store");
        let store = Store::open(&root)?;
        let fetcher = fetcher(&cache)?;
        let options = plain_names::Options { states: vec![UsJurisdiction::NorthDakota, UsJurisdiction::Nebraska],
            ..plain_names::Options::default() };
        let report = plain_names::collect(&context(&fetcher, &store)?, &options).await?;
        ensure!(report.requests == 0 && report.errors == 0, "{report:?}");
        let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
        let observed: BTreeSet<_> = schools.iter().map(|school| (school.name.as_str(), school.state)).collect();
        ensure!(observed == BTreeSet::from([("West Fargo Sheyenne High School", Some(UsJurisdiction::NorthDakota)),
            ("Adams Central", Some(UsJurisdiction::Nebraska))]));
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        for (name, school_name) in [("Zeb Noyd", "Adams Central"), ("Jaime Watson", "West Fargo Sheyenne High School")] {
            let school = schools.iter().find(|school| school.name == school_name).context("published school")?;
            let coach = coaches.iter().find(|coach| coach.name == name && coach.school == school.id).context("published coach")?;
            ensure!(coach.sport == Some(Sport::OutdoorTrack));
            ensure!(coach.evidence.first().context("coach acquisition evidence")?.observed_on == CAPTURED_AT);
            ensure!(coach.evidence.iter().all(|evidence| evidence.observed_on == CAPTURED_AT));
        }
        let physical = store.stats()?.tables;
        drop(store);
        let reopened = Store::open(&root)?;
        plain_names::collect(&context(&fetcher, &reopened)?, &options).await?;
        ensure!(reopened.scan::<CanonicalSchool>(Table::Schools)? == schools);
        ensure!(reopened.scan::<CanonicalCoach>(Table::Coaches)? == coaches);
        ensure!(reopened.stats()?.tables == physical);
        ensure!(fetcher.stats().await.physical_requests() == 0);
        Ok(())
    })
}
