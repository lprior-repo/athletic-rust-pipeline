use crate::net::{FetchError, FetchOptions, Fetcher};
use std::{collections::HashMap, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn home_campus_recorded_ajax_headers_reach_offline_capture_lookup() -> TestResult {
    admission(
        "https://www.cifsshome.org/widget/get-school-details/3374/details",
        &[
            ("x-requested-with", "XMLHttpRequest"),
            (
                "referer",
                "https://www.cifsshome.org/widget/school/directory?section=12",
            ),
        ],
    )
}

#[test]
fn arbiter_recorded_bearer_header_reaches_offline_capture_lookup() -> TestResult {
    admission(
        "https://services.arbitersports.com/api/v2/organization/public/2132/children?&pageSize=200&pageNumber=1",
        &[("Authorization", "Bearer recorded-test-credential")],
    )
}

#[test]
fn mt_recorded_bearer_header_reaches_offline_capture_lookup() -> TestResult {
    admission(
        "https://services.arbitersports.com/api/v2/organization/public/4497/children?&pageSize=200&pageNumber=1",
        &[("Authorization", "Bearer recorded-test-credential")],
    )
}

fn admission(url: &str, headers: &[(&str, &str)]) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let fetcher = Fetcher::new(
                root.path().join("http"),
                None,
                Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let options = FetchOptions {
                headers: headers
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect(),
                ..FetchOptions::default()
            };
            let result = fetcher.get(url, &options).await;
            check!(
                matches!(result, Err(FetchError::Offline { .. })),
                "legitimate source representation must reach offline lookup: {result:?}"
            );
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}

#[test]
fn ca_and_nj_ajax_representations_replay_recorded_school_details() -> TestResult {
    for (id, section, body) in [
        (
            19,
            1,
            include_str!("../../../tests/fixtures/home_campus/details_19.json"),
        ),
        (
            3374,
            12,
            include_str!("../../../tests/fixtures/home_campus/details_3374.json"),
        ),
    ] {
        let url = format!("https://www.cifsshome.org/widget/get-school-details/{id}/details");
        let headers = vec![
            ("x-requested-with".into(), "XMLHttpRequest".into()),
            (
                "referer".into(),
                format!("https://www.cifsshome.org/widget/school/directory?section={section}"),
            ),
        ];
        let captured = replay(&url, headers, body)?;
        let details = crate::home_campus::parse::parse_school_details(&captured)
            .ok_or("recorded school details must decode")?;
        check!(eq; details.profile.id, id);
    }
    Ok(())
}

#[test]
fn nh_bearer_representation_replays_recorded_children_without_storing_credential() -> TestResult {
    let url = "https://services.arbitersports.com/api/v2/organization/public/2132/children?&pageSize=200&pageNumber=1";
    let body = include_str!("../../../tests/fixtures/arbiter/nh_children_p1.body");
    let captured = replay(
        url,
        vec![(
            "Authorization".into(),
            "Bearer recorded-test-credential".into(),
        )],
        body,
    )?;
    let page = crate::arbiter::parse::parse_org_schools(&captured, url)?;
    check!(eq; page.rows.len(), 89);
    Ok(())
}

#[test]
fn nh_legacy_coach_representation_replays_after_children() -> TestResult {
    let url = "https://services.arbitersports.com/api/v2/legacy/public/2132/coaches?filter.EntityId=450&&pageSize=200&pageNumber=1";
    let captured = replay(
        url,
        vec![(
            "Authorization".into(),
            "Bearer recorded-test-credential".into(),
        )],
        include_str!("../../../tests/fixtures/arbiter/alvirne_coaches_p1.body"),
    )?;
    let page = crate::arbiter::parse::parse_coach_rows(&captured, url)?;
    check!(eq; page.rows.len(), 28);
    Ok(())
}

fn replay(
    url: &str,
    headers: Vec<(String, String)>,
    body: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use crate::net::cache::{content_digest, write_cache, CacheMeta};
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let fetcher = Fetcher::new(
                root.path().join("http"),
                None,
                Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let representation = super::representation::canonical_request(url, &headers)?;
            let key = Fetcher::key_for("GET", url, &representation.identity());
            let (body_path, meta_path) = fetcher.cache_paths(&key);
            let meta = CacheMeta {
                url: url.into(),
                method: "GET".into(),
                status: 200,
                representation,
                content_digest: content_digest(body.as_bytes()),
                bytes: body.len(),
                fetched_at: "2026-10-09T00:00:00Z".into(),
                ..CacheMeta::default()
            };
            write_cache(&body_path, &meta_path, body.as_bytes(), &meta)?;
            let journal = std::fs::read_to_string(&meta_path)?;
            check!(!journal.contains("recorded-test-credential"));
            let captured = fetcher
                .get(
                    url,
                    &FetchOptions {
                        headers,
                        ..FetchOptions::default()
                    },
                )
                .await?;
            check!(captured.from_cache);
            check!(eq; captured.body, body.as_bytes());
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(String::from_utf8(captured.body)?)
        })
}

#[test]
fn source_headers_do_not_escape_registered_endpoints_or_browser_default() -> TestResult {
    for url in [
        "https://foreign.example/api/v2/organization/public/2132/children",
        "https://services.arbitersports.com/login",
        "https://services.arbitersports.com/api/v2/organization/public/abc/children",
        "https://services.arbitersports.com:9443/api/v2/organization/public/2132/children",
        "https://www.cifsshome.org/widget/get-school-details/3374/details",
    ] {
        let headers = vec![(
            "Authorization".into(),
            "Bearer recorded-test-credential".into(),
        )];
        let refused = super::representation::canonical_request(url, &headers);
        check!(
            matches!(refused, Err(FetchError::Policy { .. })),
            "{url}: {refused:?}"
        );
        check!(matches!(
            crate::net::RepresentationHeaders::canonical(&headers),
            Err(FetchError::Policy { .. })
        ));
    }
    let headers = vec![
        ("x-requested-with".into(), "XMLHttpRequest".into()),
        (
            "referer".into(),
            "https://foreign.example/widget/school/directory".into(),
        ),
    ];
    check!(matches!(
        super::representation::canonical_request(
            "https://www.cifsshome.org/widget/get-school-details/3374/details",
            &headers
        ),
        Err(FetchError::Policy { .. })
    ));
    Ok(())
}
