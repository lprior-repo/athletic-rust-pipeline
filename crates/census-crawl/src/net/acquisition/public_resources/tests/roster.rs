use super::*;
use assertions::{exact_hop, retained, Witness};
use server::{serve, Reply};

mod intake;

const TEAM: &str = "12439";
const PATH: &str = "/teams/12439-detroit-catholic-central/roster";
const INDEX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/participation_evidence/provider_school/fixtures/mi-team-index.html"
));
const ROSTER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/participation_evidence/provider_school/fixtures/dcc-provider-roster.html"
));

#[tokio::test]
async fn owned_directory_child_uses_exact_request_and_cold_replays_original_bytes() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch = async {
        let directory = fixture.acquire().await?;
        let child = fixture
            .fetcher
            .get_public_resource_roster(
                &fixture.context(),
                &directory,
                TEAM,
                &FetchOptions::default(),
            )
            .await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((directory, child))
    };
    let (requests, (_, child)) = tokio::try_join!(
        serve(
            listener,
            [
                Reply::robots(ALLOW),
                Reply::directory(INDEX),
                roster_reply(),
            ]
        ),
        fetch
    )?;
    check!(eq; requests.len(), 3);
    check!(eq; child.capture().body, ROSTER);
    check!(!child.capture().from_cache);
    exact_hop(
        &child,
        0,
        &format!("{ORIGIN}{PATH}"),
        200,
        ROSTER.len(),
        "text/html",
    )?;
    let original = retained(&fixture.store, &child)?;
    let reference = child
        .retained_observed()?
        .ok_or("retained child")?
        .occurrence()
        .clone();
    let operation = format!(
        "public_resource_child_frontier_v1:{}",
        reference.attempt().work_id()
    );
    let receipt = fixture
        .store
        .receipt(&operation)?
        .ok_or("child frontier receipt")?;
    let before = Witness::capture(&fixture.store)?;
    fixture.clear_eligible()?;
    clear_roster_eligible(&fixture)?;
    let fixture = fixture.reopen()?;
    let directory = fixture.acquire().await?;
    let replayed = fixture
        .fetcher
        .get_public_resource_roster(
            &fixture.context(),
            &directory,
            TEAM,
            &FetchOptions::default(),
        )
        .await?;
    check!(replayed.capture().from_cache);
    check!(eq; retained(&fixture.store, &replayed)?, original);
    check!(eq; fixture.store.receipt(&operation)?, Some(receipt));
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[tokio::test]
async fn altered_child_options_or_unpublished_native_id_refuse_without_writes() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (_, directory) = tokio::try_join!(
        serve(listener, [Reply::robots(ALLOW), Reply::directory(INDEX)]),
        fetch,
    )?;
    let before = Witness::capture(&fixture.store)?;
    for options in [
        FetchOptions {
            refresh: true,
            ..FetchOptions::default()
        },
        FetchOptions {
            headers: vec![("accept".into(), "text/html".into())],
            ..FetchOptions::default()
        },
    ] {
        check!(fixture
            .fetcher
            .get_public_resource_roster(&fixture.context(), &directory, TEAM, &options,)
            .await
            .is_err());
        check!(eq; Witness::capture(&fixture.store)?, before);
    }
    for team in ["999999999", "12439/roster", ""] {
        check!(fixture
            .fetcher
            .get_public_resource_roster(
                &fixture.context(),
                &directory,
                team,
                &FetchOptions::default(),
            )
            .await
            .is_err());
        check!(eq; Witness::capture(&fixture.store)?, before);
    }
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    Ok(())
}

#[tokio::test]
async fn owned_policy_that_disallows_roster_refuses_before_child_registration() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let policy =
        b"User-agent: *\nAllow: /teams\nDisallow: /teams/12439-detroit-catholic-central/roster\n";
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (_, directory) = tokio::try_join!(
        serve(listener, [Reply::robots(policy), Reply::directory(INDEX)]),
        fetch,
    )?;
    let before = Witness::capture(&fixture.store)?;
    let result = fixture
        .fetcher
        .get_public_resource_roster(
            &fixture.context(),
            &directory,
            TEAM,
            &FetchOptions::default(),
        )
        .await;
    check!(
        matches!(result, Err(crate::CrawlError::Fetch(FetchError::Policy { detail }))
        if detail.starts_with("robots disallows "))
    );
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    Ok(())
}

#[tokio::test]
async fn foreign_owned_directory_cannot_create_roster_child_in_another_store() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let first = Fixture::new(&listener)?;
    let fetch = async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(first.acquire().await?) };
    let (_, directory) = tokio::try_join!(
        serve(listener, [Reply::robots(ALLOW), Reply::directory(INDEX)]),
        fetch,
    )?;
    let unused = TcpListener::bind("127.0.0.1:0").await?;
    let second = Fixture::new(&unused)?;
    let before = Witness::capture(&second.store)?;
    check!(second
        .fetcher
        .get_public_resource_roster(
            &second.context(),
            &directory,
            TEAM,
            &FetchOptions::default(),
        )
        .await
        .is_err());
    check!(eq; Witness::capture(&second.store)?, before);
    check!(eq; second.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

fn roster_reply() -> Reply<'static> {
    Reply {
        path: PATH,
        status: 200,
        body: ROSTER,
        content_type: "text/html",
        headers: "",
    }
}

fn clear_roster_eligible(fixture: &Fixture) -> TestResult {
    let url = format!("{ORIGIN}{PATH}");
    let request = fixture
        .fetcher
        .public_resource_request(&url, &FetchOptions::default())?;
    let representation =
        crate::net::RepresentationHeaders::from_entries(request.representation_headers().to_vec())?;
    let extra = crate::net::execute::representation::cache_extra(None, &representation);
    let key = Fetcher::key_for("GET", &url, &extra);
    let (body, metadata) = fixture.fetcher.cache_paths(&key);
    std::fs::remove_file(body)?;
    std::fs::remove_file(metadata)?;
    Ok(())
}
