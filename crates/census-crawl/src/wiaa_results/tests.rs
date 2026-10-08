use super::*;
use census_domain::model::{CompetitionLevel, GradYear, Grade};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ARCHIVE_HTML: &str = r#"
        <h3><strong>2025 Track &amp; Field State Results</strong></h3>
        <ul><li>Division 1 - <a href="/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm">Boys</a>
        | <a href="https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1girlsstateresults.htm">Girls</a></li></ul>
        <h3><strong>2025 Track &amp; Field Sectional Results</strong></h3>
        <ul><li>Neenah - <a href="/Portals/0/PDF/Results/Track/2025/neenahsectionalb.pdf">Boys</a></li></ul>
        <a href="/sports/boys-track-field/boys-track-field">Sport home</a>
    "#;

#[test]
fn archive_links_carry_year_stem_label_and_extension() -> TestResult {
    let artifacts = archive_artifacts(ARCHIVE_HTML)?;
    check!(eq;
        artifacts.len(),
        3,
        "only result-file links are artifacts: {artifacts:?}"
    );
    let first = &artifacts[0];
    check!(eq; first.year, 2025);
    check!(eq; first.stem, "d1boysstateresults");
    check!(eq; first.extension, "htm");
    check!(eq; first.label, "Boys");
    check!(
        artifacts
            .iter()
            .all(|artifact| artifact.url.starts_with("https://www.wiaawi.org/")),
        "relative hrefs are resolved against the archive host"
    );
    check!(artifacts.iter().any(|artifact| artifact.extension == "pdf"));
    Ok(())
}

#[test]
fn formats_are_decided_by_extension_and_sniffed_body() {
    assert_eq!(artifact_format("pdf", None), ArtifactFormat::Pdf);
    assert_eq!(artifact_format("rtf", None), ArtifactFormat::Unparsed);
    assert_eq!(artifact_format("txt", None), ArtifactFormat::HytekText);
    assert_eq!(
        artifact_format("htm", Some("<p>HY-TEK's Meet Manager</p>")),
        ArtifactFormat::HytekHtml
    );
    assert_eq!(
        artifact_format("htm", Some("<title>RaceDay Scoring</title>")),
        ArtifactFormat::RaceDay
    );
}

#[test]
fn current_season_files_are_found_under_the_dated_upload_path() -> TestResult {
    let body = r#"
        <a href="/sites/default/files/2026-08/trb2026d1stateresults.pdf">Boys</a>
        <a href="/sites/default/files/2026-08/tr2026arrowheadregionalindiv.pdf">Arrowhead</a>
        <a href="/sites/default/files/2026-08/tr2026beaverdamsectionalindiv.pdf">Beaver Dam</a>
        <a href="/sites/default/files/2026-08/somethingelse.pdf">Unrelated upload</a>
        <a href="/sites/default/files/2025-11/xcstate.htm">XC state</a>
    "#;
    let artifacts = archive_artifacts(body)?;
    check!(eq;
        artifacts.len(),
        5,
        "every dated upload is treated as a candidate result file: {artifacts:?}"
    );
    check!(artifacts
        .iter()
        .all(|artifact| artifact.year == 2026 || artifact.year == 2025));
    let state = artifacts
        .iter()
        .find(|artifact| artifact.stem == "trb2026d1stateresults")
        .ok_or("the 2026 state file is discovered")?;
    check!(eq; state.year, 2026);
    check!(eq; state.extension, "pdf");
    check!(eq; state.label, "Boys");
    Ok(())
}

#[test]
fn school_year_follows_the_sport_boundary() -> TestResult {
    let spring =
        school_year_for("2025-06-06", Sport::OutdoorTrack, 2025).ok_or("2025-06 is a season")?;
    check!(eq; spring.get(), 2024);
    check!(eq;
        GradYear::of(Grade::new(11).ok_or("junior grade")?, spring)
            .ok_or("11th grade in 2024 has valid grad year")?
            .get(),
        2026
    );
    let fall =
        school_year_for("2025-10-25", Sport::CrossCountry, 2025).ok_or("2025-10 is a season")?;
    check!(eq; fall.get(), 2025);
    check!(eq;
        GradYear::of(Grade::new(11).ok_or("junior grade")?, fall)
            .ok_or("11th grade in 2025 has valid grad year")?
            .get(),
        2027
    );
    check!(eq;
        school_year_for("2023", Sport::CrossCountry, 2023)
            .ok_or("2023 is a season")?
            .get(),
        2023
    );
    check!(eq;
        school_year_for("2023", Sport::OutdoorTrack, 2023)
            .ok_or("2023 is a season")?
            .get(),
        2022
    );
    Ok(())
}

#[test]
fn years_no_season_may_open_in_are_refused() {
    assert_eq!(
        school_year_for("1801-06-06", Sport::OutdoorTrack, 1801),
        None,
        "a June 1801 track meet opens the 1800-01 season, which no season may open in"
    );
    assert_eq!(
        school_year_for("no date published", Sport::CrossCountry, 1799),
        None,
        "a date-less cross-country file falls back to its archive year, which is out of window"
    );
}

#[test]
fn meet_levels_come_from_the_published_name() {
    assert_eq!(
        level_of("WIAA Track & Field State Championships"),
        CompetitionLevel::State
    );
    assert_eq!(
        level_of("WIAA D2 XC Sectionals - Boys Race"),
        CompetitionLevel::Sectional
    );
    assert_eq!(level_of("Arrowhead Regional"), CompetitionLevel::Regional);
    assert_eq!(
        level_of("Some Invitational"),
        CompetitionLevel::Invitational
    );
    assert_eq!(level_of("Dual Meet"), CompetitionLevel::Unknown);
}

fn seed_result_cache(fetcher: &crate::net::Fetcher, url: &str, body: &[u8]) -> TestResult {
    use crate::net::cache::{content_digest, write_cache, CacheMeta};
    let representation = crate::net::RepresentationHeaders::canonical(&[])?;
    let key = crate::net::Fetcher::key_for("GET", url, &representation.identity());
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    let meta = CacheMeta {
        representation,
        url: url.to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-09-19T00:00:00Z".into(),
        etag: None,
        last_modified: None,
        content_type: Some("text/html".into()),
    };
    write_cache(&body_path, &meta_path, body, &meta)?;
    Ok(())
}

#[test]
fn overlapping_archives_process_one_logical_result_once() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    use census_domain::model::{CanonicalAthlete, CanonicalPerformance, CanonicalSchool, ExactSeconds, Mark, SchoolYear};
    use census_store::Store;
    use std::time::Duration;
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let fetcher = crate::net::Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )
    ?
    .with_source("wiaa_results")
    .with_offline(true);
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Middleton", "middleton", None);
    store.append(census_store::Table::Schools, &school)?;
    std::fs::create_dir_all(store.out_dir())?;
    std::fs::write(store.out_dir().join("schools.jsonl"), format!("{}\n", serde_json::to_string(&school)?))?;
    let url = "https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm";
    let archive = format!("<a href=\"{url}\">Boys</a>");
    for (archive_url, _) in ARCHIVES {
        seed_result_cache(&fetcher, archive_url, archive.as_bytes())?;
    }
    seed_result_cache(
        &fetcher,
        url,
        include_bytes!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.htm"),
    )?;
    let context = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 school year")?,
        observed_on: "2026-09-19".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 19).ok_or("snapshot date")?,
        recording: None,
    };
    let options = Options {
        limit: None,
        refresh: false,
        observed_on: "2026-09-19".into(),
        seasons: vec![2025],
        states: vec![UsJurisdiction::Wisconsin],
        school_names: Vec::new(),
    };
    let report = collect(&context, &options).await?;
    check!(eq; report.errors, 0, "{report:?}");
    check!(eq; report.rows, 1, "{report:?}");
    check!(eq; report.from_cache, 5);
    check!(eq; report.requests, 0);
    let athletes: Vec<CanonicalAthlete> = store.scan(census_store::Table::Athletes)?;
    check!(eq; athletes.iter().map(|row| row.canonical_name.as_str()).collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["Kingston Penn"]));
    check!(athletes.iter().all(|row| row.grad_year == GradYear::CO2027));
    let performances: Vec<CanonicalPerformance> = store.scan(census_store::Table::Performances)?;
    let marks: Vec<_> = performances.iter().map(|row| row.mark.clone()).collect();
    check!(eq; marks.len(), 2);
    check!(marks.contains(&Mark::TimeSeconds(ExactSeconds::parse("10.86")?)));
    check!(marks.contains(&Mark::TimeSeconds(ExactSeconds::parse("10.89")?)));
    let physical = store.stats()?.tables;
    let replay = collect(&context, &options).await?;
    check!(eq; replay.rows, 0);
    check!(eq; store.stats()?.tables, physical);
    check!(eq; store.scan::<CanonicalPerformance>(census_store::Table::Performances)?, performances);
    Ok(())
    })
}
