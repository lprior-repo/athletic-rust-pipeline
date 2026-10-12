#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "teams_source_refusals/fixture.rs"]
mod fixture;
use census_crawl::{aia, ks, ohsaa, uhsaa, CollectionDisposition};
use fixture::{Fixture, TestResult};

#[tokio::test]
async fn az_capped_city_queries_refuse_full_inventory_without_live_fetch() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.seed(
        "https://aiaonline.org/schools/search.json?q=chandler",
        include_str!("fixtures/aia/search_chandler.json"),
    )?;
    for (id, body) in [
        (68, include_str!("fixtures/aia/school_68_seton.html")),
        (100, include_str!("fixtures/aia/school_100_chandler.html")),
        (116, include_str!("fixtures/aia/school_116_hamilton.html")),
    ] {
        fixture.seed(&format!("https://aiaonline.org/schools/{id}"), body)?;
    }
    let report = aia::collect(&fixture.context()?, &aia::Options::default()).await?;
    check!(eq; report.disposition, CollectionDisposition::Blocked);
    check!(eq; report.rejections, 1);
    check!(report
        .notes
        .iter()
        .any(|note| note.contains("capped") && note.contains("full inventory")));
    check!(!report.unfinished.is_empty());
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn ks_letter_a_capture_is_not_certified_as_full_inventory() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.seed(
        "https://kshsaa-api.kshsaa.org/directory/search/name/a/",
        include_str!("fixtures/ks/kshsaa_directory_a.json"),
    )?;
    let report = ks::collect(&fixture.context()?, &ks::Options::default()).await?;
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(eq; report.rejections, 1);
    check!(report
        .notes
        .iter()
        .any(|note| note.contains("letter a") && note.contains("full inventory")));
    check!(!report.unfinished.is_empty());
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn oh_no_association_owned_frontier_refuses_instead_of_empty_success() -> TestResult {
    let fixture = Fixture::new()?;
    let report = ohsaa::collect(&fixture.context()?, &ohsaa::Options::default()).await?;
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(eq; report.rows, 0);
    check!(eq; report.rejections, 1);
    check!(report
        .notes
        .iter()
        .any(|note| note.contains("association-owned") && note.contains("frontier")));
    check!(!report.unfinished.is_empty());
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn ut_profile_without_appointment_season_stays_explicitly_refused() -> TestResult {
    let fixture = Fixture::new()?;
    let directory = include_str!("fixtures/uhsaa/directory.html");
    fixture.seed("https://uhsaa.org/school-directory-new/", directory)?;
    let links = uhsaa::parse::parse_directory_links(directory);
    let alta = links
        .iter()
        .find(|link| link.name == "Alta Hawks")
        .ok_or("recorded Alta link")?;
    let path = alta
        .url
        .strip_prefix("../")
        .ok_or("relative Alta profile link")?;
    fixture.seed(
        &format!("https://uhsaa.org/{path}"),
        include_str!("fixtures/uhsaa/profile-alta.html"),
    )?;
    let report = uhsaa::collect(
        &fixture.context()?,
        &uhsaa::Options {
            school_names: vec!["Alta Hawks".into()],
            ..uhsaa::Options::default()
        },
    )
    .await?;
    check!(eq; report.disposition, CollectionDisposition::Blocked);
    check!(report.rows > 0);
    check!(report
        .notes
        .iter()
        .any(|note| note.contains("appointment season is not published")));
    check!(!report.unfinished.is_empty());
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
