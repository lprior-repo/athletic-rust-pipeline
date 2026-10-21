use super::{fixtures, graph, Journal, NationalCensus, Store, TestResult};
use crate::restate_services::jobs::selected_fixture::{self, Controlled};
use census_domain::review_model::ModelOptions;
use census_review::ModelClient;
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub(super) enum Boundary {
    Rules,
    Workbook,
    LostCompletion,
    RetainedRules,
    Refusal,
}

pub(super) fn controlled() -> TestResult<Controlled> {
    let controlled =
        selected_fixture::install().map_err(|error| -> Box<dyn std::error::Error> { error })?;
    let accepted = graph::read(controlled.store())?;
    let id = accepted
        .public_ids()
        .find(|id| id.as_str() == controlled.public_id())
        .ok_or("controlled public birth absent")?;
    check!(matches!(accepted.resolve_public(id)?,
        census_domain::public_identity_history::PublicResolution::Canonical(value)
            if value.public_id.as_str() == controlled.public_id()
                && value.representative_candidate.as_str() == controlled.candidate_id()
    ));
    Ok(controlled)
}

pub(super) fn journal(controlled: Controlled) -> TestResult<Journal> {
    let mut request = super::super::publication_tests::fixtures::selected_request(
        &controlled,
        &census_domain::UsJurisdiction::CENSUS_SCOPE,
    )?;
    request.pass_budget = 0;
    request.pass_delay_seconds = 0;
    let mut journal = Journal::new(&request)?;
    journal.selected_workbook = Some(controlled);
    journal.hold_stage = Some("Workbook".to_owned());
    Ok(journal)
}

pub(super) async fn exercise(mut journal: Journal, boundary: Boundary) -> TestResult<Journal> {
    let controlled = journal
        .selected_workbook
        .as_ref()
        .ok_or("selected fixture absent")?;
    let store = Arc::new(controlled.store().clone());
    let region = Arc::new(crate::spawn::Spawner::new());
    let endpoint = Endpoint::builder()
        .bind(national(Arc::clone(&store), Arc::clone(&region))?)
        .build();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async {
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let client = reqwest::Client::builder()
                .http2_prior_knowledge()
                .timeout(Duration::from_secs(5))
                .build()?;
            run(&store, &mut journal, boundary, &client, &url).await
        })
        .await;
        stop_tx
            .send(())
            .map_err(|_| "fixture endpoint lost its shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result?;
    Ok(journal)
}

fn national(store: Arc<Store>, region: Arc<crate::spawn::Spawner>) -> TestResult<NationalCensus> {
    let jobs = crate::restate_services::Jobs::new(store, region);
    let mut national = NationalCensus::new(Arc::new(fixtures::FixtureClock), jobs);
    national.review_lanes = Ok([
        ModelClient::new(
            ModelOptions::local("http://127.0.0.1:1", "fixture")?
                .with_timeout(Duration::from_millis(100)),
        )?,
        ModelClient::new(
            ModelOptions::local("http://127.0.0.1:2", "fixture")?
                .with_timeout(Duration::from_millis(100)),
        )?,
    ]);
    Ok(national)
}

async fn run(
    store: &Store,
    journal: &mut Journal,
    boundary: Boundary,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    match boundary {
        Boundary::Rules => super::deterministic::reach_rules(journal, client, url).await,
        Boundary::LostCompletion => super::deterministic::lost(store, journal, client, url).await,
        Boundary::RetainedRules => {
            super::deterministic::retained(store, journal, client, url).await
        }
        Boundary::Workbook => workbook(store, journal, client, url).await,
        Boundary::Refusal => {
            check!(journal.drive(client, url).await.is_err());
            check!(journal.output.is_none());
            Ok(())
        }
    }
}

async fn workbook(
    store: &Store,
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    journal.drive(client, url).await?;
    check!(journal.held.is_some());
    graph::summary(store, journal)?;
    let entries = journal.entries.clone();
    let rows = graph::state(store)?;
    let accepted = graph::read(store)?;
    let replay = journal.step(client, url).await?;
    check!(!replay.iter().any(|entry| entry.kind == 0x0411));
    check!(eq; journal.entries, entries);
    graph::unchanged(store, &accepted, &[])?;
    check!(eq; graph::state(store)?, rows);
    journal.release()?;
    journal.drive(client, url).await?;
    check!(journal.output.is_some());
    Ok(())
}
