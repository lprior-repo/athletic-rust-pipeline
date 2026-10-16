use super::super::{CanonicalPerformance, TestResult};
use census_crawl::milesplit::{
    collect_result_sets, ResultSetOptions, ResultSetRef, ResultSetRequest,
};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{AdapterContext, AdapterReport, CaptureContext, CrawlResult};
use census_domain::model::{CanonicalSchool, SchoolYear, SourceIdentity, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, future::Future, pin::Pin};

pub(in super::super) struct ResultCase {
    pub id: u64,
    pub meet: &'static str,
    pub result_set: &'static str,
    pub date: &'static str,
    pub name: &'static str,
    pub mark: &'static str,
    pub units: &'static str,
}

pub(in super::super) const CASES: [ResultCase; 2] = [
    ResultCase {
        id: 201782263,
        meet: "725218",
        result_set: "1266814",
        date: "2026-03-27",
        name: "Synthetic Michigan March Meet",
        mark: "13-9",
        units: "165000",
    },
    ResultCase {
        id: 201782264,
        meet: "725219",
        result_set: "1266815",
        date: "2026-04-03",
        name: "Synthetic Michigan April Meet",
        mark: "14-9",
        units: "177000",
    },
];

impl ResultCase {
    pub fn url(&self) -> String {
        format!(
            "https://mi.milesplit.com/meets/{}/results/{}/raw",
            self.meet, self.result_set
        )
    }
}

pub(super) async fn append(store: &Store) -> TestResult<[CanonicalPerformance; 2]> {
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        "Detroit Catholic Central",
        "detroit catholic central",
        None,
    )
    .0;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "38332",
    ));
    store.append(Table::Schools, &school)?;
    let [first, second] = &CASES;
    Ok([
        append_one(store, first).await?,
        append_one(store, second).await?,
    ])
}

async fn append_one(store: &Store, case: &ResultCase) -> TestResult<CanonicalPerformance> {
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let reference = ResultSetRef::parse(&case.url()).ok_or("synthetic result set")?;
    let request = census_crawl::milesplit::owned_meet_request(
        &fetcher,
        &reference,
        &FetchOptions::default(),
    )?;
    let owned = json!({"_meta": {"status_code": 200}, "_embedded": {"meet": {"id": case.meet}},
        "data": [{"id": case.id, "athleteId": "14222592", "teamId": "38332",
            "teamName": "Detroit Catholic Central", "meetId": case.meet, "meetResultsId": case.result_set, "gender": "M",
            "units": case.units, "place": "1", "heat": "1", "firstName": "Synthetic",
            "lastName": "Participant", "gradYear": "2027", "eventName": "Long Jump",
            "roundName": "Finals", "round": "f", "mark": case.mark, "divisionName": "High School",
            "profileUrl": "https://mi.milesplit.com/athletes/14222592-synthetic-participant"}]});
    seed(
        &fetcher,
        request.original_url(),
        &serde_json::to_vec(&owned)?,
    )?;
    seed(&fetcher, &reference.url, raw_body(case).as_bytes())?;
    let context = AdapterContext {
        capture: CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7).ok_or("date")?,
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
    check!(eq; acquisition.await?.errors, 0);
    Ok(store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .find(|row| row.source_key == format!("milesplit_result:{}", case.id))
        .ok_or("synthetic retained result")?)
}

pub(in super::super) fn raw_body(case: &ResultCase) -> String {
    let header = format!(
        "  Pl {:<26} {:<3} {:<30}{:>15}",
        "Name", "Yr", "Team", "Mark"
    );
    let row = format!(
        "   1 {:<26} {:<3} {:<30}{:>15}",
        "Synthetic Participant", "11", "Detroit Catholic Central", case.mark
    );
    format!(
        r#"<script type="application/ld+json">{{"@context":"https://schema.org","@type":"SportsEvent","name":"{}","description":"Wholly synthetic MI acceptance fixture","startDate":"{}","endDate":"{}","homeTeam":"Detroit Catholic Central","sport":"Outdoor Track & Field","location":{{"@type":"Place","name":"Synthetic Michigan Venue","address":{{"@type":"PostalAddress","addressLocality":"Novi","addressRegion":"MI"}}}},"url":"{}"}}</script><link rel="canonical" href="{}"><pre>Boys Long Jump Finals
{header}
{row}
</pre>"#,
        case.name,
        case.date,
        case.date,
        case.url(),
        case.url()
    )
}

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult {
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("GET\u{1f}{url}\u{1f}").as_bytes())
    );
    let key = digest.get(..32).ok_or("cache key")?;
    let metadata = json!({"url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body)), "bytes": body.len(),
        "fetched_at": "2026-10-01T23:44:16Z", "content_type": null});
    std::fs::create_dir_all(fetcher.cache_dir())?;
    std::fs::write(fetcher.cache_dir().join(format!("{key}.body")), body)?;
    std::fs::write(
        fetcher.cache_dir().join(format!("{key}.meta.json")),
        serde_json::to_vec(&metadata)?,
    )?;
    Ok(())
}
