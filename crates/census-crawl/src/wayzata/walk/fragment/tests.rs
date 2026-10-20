use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::recording::Recording;
use census_store::Store;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn seed(fetcher: &crate::net::Fetcher, url: &str, body: &[u8]) -> TestResult {
    let representation = crate::net::RepresentationHeaders::canonical(&[])?;
    let key = crate::net::Fetcher::key_for("GET", url, &representation.identity());
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            redirects: Vec::new(),
            representation,
            url: url.into(),
            response_url: None,
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: "2026-09-20T14:39:00Z".into(),
            etag: None,
            last_modified: None,
            content_type: Some("text/html".into()),
        },
    )?;
    Ok(())
}

#[test]
fn wayzata_frozen_cursor_restores_original_body_without_rereading_mutable_schedule() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let capture = crate::net::durable_capture_context(&root.path().join("capture"))?;
            let store = Store::open(root.path())?;
            let fetcher = crate::net::Fetcher::new(
                store.http_cache_dir(),
                None,
                std::time::Duration::ZERO,
                std::collections::HashMap::new(),
                Vec::new(),
            )?
            .with_source("wayzata")
            .with_offline(true);
            let track =
                include_bytes!("../../../../tests/fixtures/wayzata/track_2026_schedule.html");
            let xc = include_bytes!("../../../../tests/fixtures/wayzata/xc_2026_schedule.html");
            seed(&fetcher, &schedule_url(ScheduleSport::Track, 2026), track)?;
            seed(
                &fetcher,
                &schedule_url(ScheduleSport::CrossCountry, 2026),
                xc,
            )?;
            let recording = Recording::spooled(store.clone(), "wayzata-frozen/0".into())?;
            let ctx = AdapterContext {
                capture: crate::CaptureContext::DurableV1(&capture),
                fetcher: &fetcher,
                store: &store,
                recording: Some(&recording),
                refresh: false,
                school_year: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
                observed_on: "2026-09-20".into(),
                performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7).ok_or("as of")?,
            };
            let options = Options {
                years: vec![2026],
                jurisdictions: vec![census_domain::UsJurisdiction::Minnesota],
                limit: None,
                refresh: false,
                observed_on: Some("2026-09-20".into()),
            };
            let frozen = schedules(&ctx, &options).await?;
            check!(eq; frozen.len(), 2);
            for sport in [ScheduleSport::Track, ScheduleSport::CrossCountry] {
                seed(
                    &fetcher,
                    &schedule_url(sport, 2026),
                    b"<html>live catalogue replaced</html>",
                )?;
            }
            let resumed = Recording::spooled(store.clone(), "wayzata-frozen/1".into())?;
            let ctx = AdapterContext {
                recording: Some(&resumed),
                ..ctx
            };
            let before = fetcher.stats().await.cache_hits;
            let restored = schedules(&ctx, &options).await?;
            let track_capture = restored.into_iter().next().ok_or("frozen track page")?;
            check!(eq; track_capture.capture.rows, 13);
            let fetched = capture::read(&ctx, track_capture.capture)?;
            check!(eq; fetched.body, track.to_vec());
            collect_fragment(&ctx, &options, 1).await?;
            check!(eq; fetcher.stats().await.cache_hits, before);
            Ok(())
        })
}
