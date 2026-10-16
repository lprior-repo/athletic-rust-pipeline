use census_crawl::milesplit::{
    collect_result_sets, ResultSetOptions, ResultSetRef, ResultSetRequest, Site,
};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{AdapterContext, AdapterReport, CaptureContext, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, CanonicalSchool, Gender, SchoolYear, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

const MEET: &str = "726001";
const RESULT_SET: &str = "1267001";
const RESULT_ID: u64 = 201783001;
const MEET_NAME: &str = "Synthetic Michigan Girls Meet";
const MEET_DATE: &str = "2026-04-17";
const FETCHED_AT: &str = "2026-09-20T12:00:00Z";

pub(super) async fn append(
    store: &Store,
    school: &CanonicalSchool,
    athlete: &CanonicalAthlete,
) -> TestResult<CanonicalPerformance> {
    let owner = school
        .source_identities
        .iter()
        .find(|source| source.namespace == SourceNamespace::MilesplitSchool)
        .ok_or("selected fixture provider school")?;
    let person = athlete
        .source
        .as_ref()
        .ok_or("selected fixture provider athlete")?;
    let site = Site::for_jurisdiction(UsJurisdiction::Michigan);
    let (first, last) = athlete
        .canonical_name
        .split_once(' ')
        .ok_or("selected fixture athlete requires first and last names")?;
    let gender = match athlete.gender {
        Gender::Girls => "F",
        Gender::Boys => "M",
        Gender::Mixed | Gender::Unknown => {
            return Err("selected fixture athlete requires a binary competition category".into());
        }
    };
    let row = json!({
        "id": RESULT_ID,
        "athleteId": person.id,
        "teamId": owner.id,
        "teamName": school.name,
        "meetId": MEET,
        "meetResultsId": RESULT_SET,
        "gender": gender,
        "place": "1",
        "heat": "1",
        "firstName": first,
        "lastName": last,
        "gradYear": athlete.grad_year.get().to_string(),
        "eventName": "100 Meter Dash",
        "eventCode": "100m",
        "eventDistance": 100,
        "roundName": "Finals",
        "round": "f",
        "mark": "13.00",
        "divisionName": "High School",
        "profileUrl": super::athlete_profile_url(&site.host(), &person.id, &athlete.canonical_name),
    });
    let owned = json!({
        "_meta": {"status_code": 200},
        "_embedded": {"meet": {"id": MEET}},
        "data": [row],
    });
    let url = format!(
        "https://{}/meets/{}/results/{}/raw",
        site.host(),
        MEET,
        RESULT_SET
    );
    ingest(
        store,
        &url,
        &serde_json::to_vec(&owned)?,
        &raw_body(&url, school, athlete)?,
    )
    .await
}

async fn ingest(
    store: &Store,
    url: &str,
    owned: &[u8],
    raw: &[u8],
) -> TestResult<CanonicalPerformance> {
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let reference = ResultSetRef::parse(url).ok_or("selected fixture result set")?;
    let request = census_crawl::milesplit::owned_meet_request(
        &fetcher,
        &reference,
        &FetchOptions::default(),
    )?;
    seed(&fetcher, request.original_url(), owned)?;
    seed(&fetcher, &reference.url, raw)?;
    let context = AdapterContext {
        capture: CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("selected fixture school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7)
            .ok_or("selected fixture as of")?,
        recording: None,
    };
    let options = ResultSetOptions {
        urls: vec![ResultSetRequest {
            url: reference.url,
            jurisdiction: UsJurisdiction::Michigan,
        }],
    };
    let acquisition: Pin<Box<dyn Future<Output = CrawlResult<AdapterReport>> + '_>> =
        Box::pin(collect_result_sets(&context, &options));
    let report = acquisition.await?;
    check!(eq; report.errors, 0);
    store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .find(|row| row.source_key == format!("milesplit_result:{RESULT_ID}"))
        .ok_or("selected fixture retained owned performance".into())
}

fn raw_body(
    url: &str,
    school: &CanonicalSchool,
    athlete: &CanonicalAthlete,
) -> TestResult<Vec<u8>> {
    let header = format!(
        "  Pl {:<26} {:<3} {:<30}{:>15}",
        "Name", "Yr", "Team", "Mark"
    );
    let row = format!(
        "   1 {:<26} {:<3} {:<30}{:>15}",
        athlete.canonical_name, "11", school.name, "13.00"
    );
    Ok(format!(
        r#"<script type="application/ld+json">{{"@context":"https://schema.org","@type":"SportsEvent","name":"{MEET_NAME}","description":"Wholly synthetic MI selected publication fixture","startDate":"{MEET_DATE}","endDate":"{MEET_DATE}","homeTeam":"{school}","sport":"Outdoor Track & Field","location":{{"@type":"Place","name":"Synthetic Michigan Venue","address":{{"@type":"PostalAddress","addressLocality":"Novi","addressRegion":"MI"}}}},"url":"{url}"}}</script><link rel="canonical" href="{url}"><pre>Girls 100 Meter Dash Finals
{header}
{row}
</pre>"#,
        school = school.name,
    )
    .into_bytes())
}

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult<()> {
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("GET\u{1f}{url}\u{1f}").as_bytes())
    );
    let key = digest.get(..32).ok_or("selected fixture cache key")?;
    let metadata = json!({"url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body)), "bytes": body.len(),
        "fetched_at": FETCHED_AT, "content_type": null});
    std::fs::create_dir_all(fetcher.cache_dir())?;
    std::fs::write(fetcher.cache_dir().join(format!("{key}.body")), body)?;
    std::fs::write(
        fetcher.cache_dir().join(format!("{key}.meta.json")),
        serde_json::to_vec(&metadata)?,
    )?;
    Ok(())
}
