#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::milesplit;
use census_crawl::milesplit::boundary;
use census_crawl::net::{FetchOptions, Fetcher};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

const WI_TEAMS_FIXTURE: &str = include_str!("fixtures/milesplit/wi_teams_index.html");
const WI_ROSTER_FIXTURE: &str = include_str!("fixtures/milesplit/wi_roster_52649.html");

fn seed_cache(cache: &std::path::Path, url: &str, body: &str) -> anyhow::Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache.join(format!("{key}.body")), body)?;
    let meta = serde_json::json!({
        "url": url,
        "response_url": null,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": "text/html; charset=utf-8",
    });
    std::fs::write(cache.join(format!("{key}.meta.json")), meta.to_string())?;
    Ok(())
}

struct RecordingHook {
    reached: Arc<Mutex<Vec<boundary::Point>>>,
}

impl RecordingHook {
    fn new() -> Self {
        Self {
            reached: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn take(&self) -> Vec<boundary::Point> {
        match self.reached.lock() {
            Ok(mut guard) => guard.drain(..).collect(),
            Err(_) => Vec::new(),
        }
    }
}

impl boundary::Hook for RecordingHook {
    fn reached<'a>(
        &'a self,
        point: boundary::Point,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), census_crawl::CrawlError>> + Send + 'a>,
    > {
        if let Ok(mut guard) = self.reached.lock() {
            guard.push(point);
        }
        Box::pin(async move { Ok(()) })
    }
}

#[test]
fn fetch_roster_fires_all_boundary_points_in_order() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = census_store::Store::open(dir.path())?;
            let teams = milesplit::parse_team_index(WI_TEAMS_FIXTURE)?.teams;
            let team = teams
                .first()
                .ok_or(anyhow::anyhow!("index fixture lists no teams"))?
                .clone();
            let site = milesplit::Site::for_jurisdiction(census_domain::UsJurisdiction::Wisconsin);
            seed_cache(&store.http_cache_dir(), &site.teams_url(), WI_TEAMS_FIXTURE)?;
            let roster_body = format!(
                "<script type=\"application/ld+json\">{{\"@type\":\"WebPage\",\"url\":\"{}/roster\"}}</script>\n{WI_ROSTER_FIXTURE}",
                team.url
            );
            seed_cache(
                &store.http_cache_dir(),
                &format!("{}/roster", team.url),
                &roster_body,
            )?;
            let fetcher = Fetcher::new(
                store.http_cache_dir(),
                None,
                std::time::Duration::from_millis(1),
                std::collections::HashMap::new(),
                Vec::new(),
            )?;
            let hook = RecordingHook::new();
            let parsed = milesplit::parse_roster(WI_ROSTER_FIXTURE, team.clone())?;
            let roster = parsed
                .roster()
                .ok_or(anyhow::anyhow!("roster fixture carries no readable athletes"))?;
            let expected_chunks = roster.athletes.len().div_ceil(boundary::CHUNK_ROWS);
            let outcome =
                milesplit::fetch_roster(&fetcher, &team, &FetchOptions::default(), Some(&hook))
                    .await?;
            let captured = outcome
                .verdict
                .roster()
                .ok_or(anyhow::anyhow!("crawl verdict carries no roster"))?;
            check!(
                eq;
                captured.athletes.len(),
                roster.athletes.len(),
                "every parsed athlete must be captured"
            );
            let reached = hook.take();
            let kinds: Vec<String> = reached
                .iter()
                .map(|point| point.kind().to_string())
                .collect();
            let expected_kinds: Vec<String> = {
                let mut v = Vec::new();
                v.push("before_source_request".to_string());
                v.push("response_received".to_string());
                v.push("capture_committed".to_string());
                for _ in 0..expected_chunks {
                    v.push("page_chunk".to_string());
                }
                v
            };
            check!(eq; kinds, expected_kinds, "point kinds must match expected sequence");
            let mut seen = HashSet::new();
            let mut last_index = -1i64;
            for (i, point) in reached.iter().enumerate() {
                match point {
                    boundary::Point::BeforeSourceRequest => check!(eq; i, 0),
                    boundary::Point::ResponseReceived => check!(eq; i, 1),
                    boundary::Point::CaptureCommitted => check!(eq; i, 2),
                    boundary::Point::PageChunk { index } => {
                        if i < 3 {
                            return Err(anyhow::anyhow!(
                                "a page chunk arrived before the response was received"
                            ));
                        }
                        check!(eq; i64::from(*index), last_index.saturating_add(1));
                        last_index = i64::from(*index);
                    }
                    boundary::Point::BeforeApply
                    | boundary::Point::AfterCommitBeforeAck => {
                        return Err(anyhow::anyhow!(
                            "service-side points must not fire in the crawl crate"
                        ));
                    }
                }
                seen.insert(point.kind());
            }
            for kind in [
                "before_source_request",
                "response_received",
                "capture_committed",
                "page_chunk",
            ] {
                if !seen.contains(kind) {
                    return Err(anyhow::anyhow!("boundary point kind {kind} never fired"));
                }
            }
            check!(eq; reached.len(), 3 + expected_chunks);
            Ok(())
        })
}
