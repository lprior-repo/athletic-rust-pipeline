use super::parse_roster;
use crate::milesplit::parse::parse_team_index;
use crate::milesplit::wire::TeamRef;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchOptions, Fetcher};
use std::collections::HashMap;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TEAMS: &str = include_str!("../../../../tests/fixtures/milesplit/wi_teams_index.html");
const ABBOTSFORD: &str = include_str!("../../../../tests/fixtures/milesplit/wi_roster_52649.html");

fn team(id: &str) -> TestResult<TeamRef> {
    Ok(parse_team_index(TEAMS)?
        .into_iter()
        .find(|team| team.id == id)
        .ok_or("exact provider school in captured index")?)
}

fn conflicting_body() -> TestResult<String> {
    Ok(format!(
        "<link rel=\"canonical\" href=\"{}/roster\" />\n{ABBOTSFORD}",
        team("26848")?.url
    ))
}

fn assert_no_admitted_roster(
    outcome: crate::CrawlResult<crate::milesplit::RosterVerdict>,
) -> TestResult {
    if let Ok(verdict) = outcome {
        check!(
            verdict
                .roster()
                .is_none_or(|roster| roster.athletes.is_empty()),
            "contradictory document ownership must not admit captured athletes: {verdict:?}"
        );
    }
    Ok(())
}

#[test]
fn conflicting_canonical_owner_cannot_project_abbotsford_affiliations() -> TestResult {
    let requested = team("52649")?;
    check!(eq; requested.name, "Abbotsford");
    check!(eq; team("26848")?.name, "Abundant Life Christian");
    assert_no_admitted_roster(parse_roster(&conflicting_body()?, requested))?;
    Ok(())
}

#[test]
fn captured_foreign_canonical_owner_admits_no_abbotsford_roster() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = Fetcher::new(
                dir.path().join("http"),
                None,
                Duration::ZERO,
                HashMap::new(),
                vec!["milesplit.com".into()],
            )?
            .with_offline(true);
            let requested = team("52649")?;
            let body = conflicting_body()?;
            let url = format!("{}/roster", requested.url);
            let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", &url, ""));
            write_cache(
                &body_path,
                &meta_path,
                body.as_bytes(),
                &CacheMeta {
                    representation: crate::net::RepresentationHeaders::default(),
                    url: url.clone(),
                    method: "GET".into(),
                    status: 200,
                    content_digest: content_digest(body.as_bytes()),
                    bytes: body.len(),
                    fetched_at: "2026-10-02T00:00:00Z".into(),
                    ..CacheMeta::default()
                },
            )?;
            let outcome = crate::milesplit::fetch_roster(
                &fetcher,
                &requested,
                &FetchOptions::default(),
                None,
            )
            .await;
            if let Ok(captured) = &outcome {
                check!(eq; captured.capture.url, url);
                check!(eq; captured.capture.body, body.as_bytes());
                check!(eq;
                    captured.capture.content_digest,
                    content_digest(body.as_bytes())
                );
                check!(eq; captured.capture.fetched_at, "2026-10-02T00:00:00Z");
                check!(captured.capture.from_cache);
            }
            assert_no_admitted_roster(outcome.map(|captured| captured.verdict))?;
            Ok(())
        })
}
