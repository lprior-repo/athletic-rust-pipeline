use super::{collect, Options};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::{net::Fetcher, AdapterContext};
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachTenure, SchoolYear};
use census_store::{Store, Table};
use std::collections::HashMap;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const DIRECTORY: &str = "https://uhsaa.org/school-directory-new/";
const PROFILE: &str = "https://uhsaa.org/school-directory/?id=Alta&Reg=6&schoolID=1";
const CAPTURED: &str = "2026-10-04T00:00:00Z";
const BODY: &[u8] = include_bytes!("../../tests/fixtures/uhsaa/profile-alta.html");

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult {
    let key = Fetcher::key_for("GET", url, "");
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            redirects: Vec::new(),
            representation: crate::net::RepresentationHeaders::default(),
            url: url.to_string(),
            response_url: None,
            method: "GET".to_string(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: CAPTURED.to_string(),
            etag: None,
            last_modified: None,
            content_type: Some("text/html".to_string()),
        },
    )?;
    Ok(())
}

#[test]
fn uhsaa_retained_capture_readback_keeps_owned_address_and_unknown_tenure() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(root.path().join("http"), None, Duration::ZERO,
            HashMap::new(), Vec::new())?.with_offline(true);
        seed(&fetcher, DIRECTORY, include_bytes!("../../tests/fixtures/uhsaa/directory.html"))?;
        seed(&fetcher, PROFILE, BODY)?;
        let school_year = SchoolYear::new(2026).ok_or("school year")?;
        let context = AdapterContext {
            capture: crate::CaptureContext::LegacyMetadata,
            fetcher: &fetcher, store: &store, refresh: false, school_year,
            observed_on: "2026-10-12".to_string(),
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 12).ok_or("date")?,
            recording: None,
        };
        let options = Options { school_names: vec!["Alta Hawks".to_string()], ..Options::default() };
        let report = collect(&context, &options).await?;
        check!(eq; report.requests, 0);
        check!(eq; report.rows, 1);
        let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
        let school = schools.first().ok_or("school readback")?;
        check!(eq; school.postal_addresses.len(), 1, "published street must survive as an owned typed claim");
        let claim = school.postal_addresses.first().ok_or("postal readback")?;
        check!(eq; claim.owner().id.as_str(), "1");
        check!(eq; claim.capture_sha256(), content_digest(BODY));
        check!(eq; claim.evidence().observed_on.as_str(), CAPTURED);
        check!(eq; claim.evidence().source.url.as_deref(), Some(PROFILE));
        check!(eq; claim.address().line1().map(|value| value.as_str()), Some("11055 S. 1000 E."));
        check!(eq; claim.address().city().map(|value| value.as_str()), Some("Sandy"));
        check!(eq; school.city.as_deref(), Some("Sandy"));
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        check!(eq; coaches.len(), 4);
        for coach in &coaches {
            check!(eq; coach.tenure_state(school_year)?, CoachTenure::Unknown);
            let fact = coach.tenure_evidence.first().ok_or("explicit unknown tenure")?;
            check!(eq; fact.tenure, CoachTenure::Unknown);
            check!(eq; fact.source_sha256.as_str(), content_digest(BODY));
            check!(eq; fact.retrieved_at.as_str(), CAPTURED);
        }
        check!(eq; report.rejections, 1);
        check!(eq; report.disposition, crate::CollectionDisposition::Blocked);
        let replay = collect(&context, &options).await?;
        check!(eq; replay.rejections, 1);
        check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?, schools);
        check!(eq; store.scan::<CanonicalCoach>(Table::Coaches)?, coaches);
        Ok(())
    })
}

#[test]
fn uhsaa_american_fork_space_href_uses_encoded_capture_url() -> TestResult {
    const FORK_RAW: &str = "https://uhsaa.org/school-directory/?id=American Fork&Reg=3&schoolID=3";
    const FORK_ENCODED: &str =
        "https://uhsaa.org/school-directory/?id=American%20Fork&Reg=3&schoolID=3";
    const FORK_BODY: &[u8] = br#"<html><body><div class="school-info"><h1 class="school-name">American Fork Cavemen</h1><ul class="school-details"><li><i class="fas fa-map-marker-alt"></i> 123 E Main St, American Fork, UT 84003</li><li>District: Alpine</li><li>Classification: 6A</li><li>Region: 3</li></ul></div><table class="staff-table"><tbody><tr><td>Boys Cross Country</td><td><a href="mailto:coach@example.org">Test Coach</a></td><td></td></tr></tbody></table></body></html>"#;
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(root.path().join("http"), None, Duration::ZERO,
            HashMap::new(), Vec::new())?.with_offline(true);
        seed(&fetcher, DIRECTORY, include_bytes!("../../tests/fixtures/uhsaa/directory.html"))?;
        seed(&fetcher, FORK_ENCODED, FORK_BODY)?;
        let school_year = SchoolYear::new(2026).ok_or("school year")?;
        let context = AdapterContext {
            capture: crate::CaptureContext::LegacyMetadata,
            fetcher: &fetcher, store: &store, refresh: false, school_year,
            observed_on: "2026-10-12".to_string(),
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 12).ok_or("date")?,
            recording: None,
        };
        let options = Options { school_names: vec!["American Fork Cavemen".to_string()], ..Options::default() };
        let report = collect(&context, &options).await?;
        check!(eq; report.rows, 1);
        check!(eq; report.errors, 0, "encoded American Fork capture must not report source errors: {report:?}");
        let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
        let school = schools.first().ok_or("school readback")?;
        check!(eq; school.postal_addresses.len(), 1);
        let claim = school.postal_addresses.first().ok_or("postal readback")?;
        check!(eq; claim.owner().id.as_str(), "3");
        check!(eq; claim.capture_sha256(), content_digest(FORK_BODY));
        check!(eq; claim.evidence().observed_on.as_str(), CAPTURED);
        let url = claim.evidence().source.url.as_deref().ok_or("capture url")?;
        check!(eq; url, FORK_ENCODED);
        check!(!url.chars().any(char::is_whitespace), "retained capture URL retains whitespace: {url:?}");
        check!(eq; claim.address().line1().map(|value| value.as_str()), Some("123 E Main St"));
        check!(eq; claim.address().city().map(|value| value.as_str()), Some("American Fork"));
        check!(eq; super::owner::school_id(FORK_RAW).map_err(|error| error.to_string())?.as_str(), "3");
        check!(eq; super::owner::school_id(url).map_err(|error| error.to_string())?.as_str(), "3");
        Ok(())
    })
}
