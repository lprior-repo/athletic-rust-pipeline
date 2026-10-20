use super::super::publication_tests::codec;
use super::{
    fixtures, AppliedAthleteIdentity, Journal, ReviewCase, ReviewState, Store, Table, TestResult,
};
use census_domain::model::{CanonicalAthlete, IdentityStatus, SchoolYear, ATHLETE_IDENTITY_FAMILY};
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

enum Boundary {
    RefusedPublication,
    LostCompletion,
    CaptureRules,
    HistoricalRules(Box<Journal>),
}

#[tokio::test]
async fn refused_deterministic_publication_cannot_release_advice_or_historical_closure(
) -> TestResult {
    exercise(Boundary::RefusedPublication).await.map(|_| ())
}

#[tokio::test]
async fn committed_deterministic_publication_replays_after_its_run_completion_is_lost() -> TestResult
{
    exercise(Boundary::LostCompletion).await.map(|_| ())
}

#[tokio::test]
async fn completed_historical_rules_replay_the_same_journal_without_retroactive_closure(
) -> TestResult {
    let journal = exercise(Boundary::CaptureRules).await?;
    exercise(Boundary::HistoricalRules(Box::new(journal)))
        .await
        .map(|_| ())
}

async fn exercise(boundary: Boundary) -> TestResult<Journal> {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);
    let rows = fixtures::seed(&store)?;
    let old = obsolete(&store, &rows)?;
    if matches!(&boundary, Boundary::RefusedPublication) {
        store.journal_done(
            "identity_retained_supersession_v1",
            &old.id,
            &serde_json::json!(["foreign successor", "foreign reason"]),
        )?;
    }
    let historical_summary = if matches!(&boundary, Boundary::HistoricalRules(_)) {
        Some(census_review::reconcile_athletes(&store, "2026-10-12", false)?.summary())
    } else {
        None
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let mut national = super::NationalCensus::new(Arc::new(fixtures::FixtureClock), jobs);
    national.review_lanes = Ok([
        census_review::ModelClient::new(
            census_domain::review_model::ModelOptions::local("http://127.0.0.1:1", "fixture")?
                .with_timeout(Duration::from_millis(100)),
        )?,
        census_review::ModelClient::new(
            census_domain::review_model::ModelOptions::local("http://127.0.0.1:2", "fixture")?
                .with_timeout(Duration::from_millis(100)),
        )?,
    ]);
    let endpoint = Endpoint::builder().bind(national).build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = tokio::time::timeout(Duration::from_secs(30), async move {
            let client = reqwest::Client::builder()
                .http2_prior_knowledge()
                .timeout(Duration::from_secs(5))
                .build()?;
            let controlled = crate::restate_services::jobs::selected_fixture::install()
                .map_err(|error| error.to_string())?;
            let mut request = super::super::publication_tests::fixtures::selected_request(
                &controlled,
                &census_domain::UsJurisdiction::CENSUS_SCOPE,
            )?;
            request.pass_budget = 0;
            request.pass_delay_seconds = 0;
            let mut journal = Journal::new(&request)?;
            journal.selected_workbook = Some(controlled);
            journal.hold_stage = Some("Workbook".to_owned());
            match boundary {
                Boundary::RefusedPublication => {
                    refused(&store, &old, &mut journal, &client, &url).await?;
                }
                Boundary::LostCompletion => {
                    replay(&store, &old, &mut journal, &client, &url).await?;
                }
                Boundary::CaptureRules => reach_rules(&mut journal, &client, &url).await?,
                Boundary::HistoricalRules(recorded) => {
                    journal = *recorded;
                    let summary = historical_summary.ok_or("historical summary absent")?;
                    historical(&store, &old, &summary, &mut journal, &client, &url).await?;
                }
            }
            Ok::<_, Box<dyn std::error::Error>>(journal)
        })
        .await;
        stop_tx
            .send(())
            .map_err(|_| "fixture endpoint lost its shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result
}

fn obsolete(store: &Store, rows: &[CanonicalAthlete]) -> TestResult<ReviewCase> {
    let first = rows.first().ok_or("fixture subject absent")?;
    let members = rows.iter().map(|row| row.id.cast()).collect::<Vec<_>>();
    let evidence = store.athlete_identity_index()?.case_evidence(
        "Synthetic Runner",
        "obsolete machine finding",
        &members,
    )?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        "Synthetic Runner",
        "obsolete machine finding",
        evidence,
    );
    let (binding, digest) = case.id.rsplit_once(':').ok_or("fixture case digest")?;
    let (subject, _) = binding.rsplit_once(":p").ok_or("fixture case policy")?;
    case.id = format!("{subject}:p1:{digest}");
    case.state = ReviewState::Retained;
    case.member_ids = members;
    store.replace(Table::ReviewCases, &case)?;
    Ok(case)
}

fn run_names(journal: &Journal) -> TestResult<Vec<String>> {
    journal
        .entries
        .iter()
        .filter(|entry| entry.kind == 0x0411)
        .try_fold(Vec::new(), |mut names, entry| {
            if let Some(name) = codec::field(&entry.payload, 12)? {
                let name = std::str::from_utf8(name)?;
                if name.starts_with("identity:") {
                    names.push(name.to_owned());
                }
            }
            Ok(names)
        })
}

async fn refused(
    store: &Store,
    old: &ReviewCase,
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    let failure = journal
        .drive(client, url)
        .await
        .err()
        .ok_or("refused publication released national work")?;
    println!("refused deterministic publication: {failure}");
    let names = run_names(journal)?;
    check!(names.contains(&"identity:rules".to_owned()));
    check!(!names
        .iter()
        .any(|name| name.starts_with("identity:advice:")));
    check!(eq; store.derived_generation(), 0);
    check!(store.scan::<ReviewCase>(Table::ReviewCases)?.contains(old));
    check!(eq; store.journal_payload("identity_retained_supersession_v1", &old.id)?,
        Some(serde_json::json!(["foreign successor", "foreign reason"])));
    let applications = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; applications.len(), 1);
    let receipts = store.receipt_count()?;
    let season = SchoolYear::new(2026).ok_or("fixture season")?;
    for _ in 0..2 {
        let result =
            census_reconcile::index::derive(store, "fixture:identity:rules", "2026-10-12", season);
        check!(matches!(
            result,
            Err(census_report::report::ReportError::Store(
                census_store::StoreError::PublicationRefused { .. }
            ))
        ));
        check!(eq; store.derived_generation(), 0);
        check!(eq; store.receipt_count()?, receipts);
        check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, applications);
        check!(store.scan::<ReviewCase>(Table::ReviewCases)?.contains(old));
    }
    Ok(())
}

async fn replay(
    store: &Store,
    old: &ReviewCase,
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    reach_rules(journal, client, url).await?;
    check!(!run_names(journal)?
        .iter()
        .any(|name| name.starts_with("identity:advice:")));
    let applications = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; applications.len(), 1);
    let pointer = store.derived_generation();
    check!(pointer > 0);
    let receipts = store.receipt_count()?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    let mut closed = old.clone();
    closed.state = ReviewState::Superseded;
    check!(cases.contains(&closed));
    let witness = store.journal_payload("identity_retained_supersession_v1", &old.id)?;
    check!(witness.is_some());
    let projection = store.athlete_identity_projection()?;
    for member in &old.member_ids {
        check!(eq; projection.status(member.as_str())?, IdentityStatus::Verified);
    }
    lose_completion(journal)?;
    let replayed = journal.step(client, url).await?;
    check!(replayed.iter().any(|entry| entry.kind == 0x0005));
    check!(!replayed.iter().any(|entry| entry.kind == 0x0411));
    check!(eq; store.derived_generation(), pointer);
    check!(eq; store.receipt_count()?, receipts);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, applications);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, cases);
    check!(eq; store.journal_payload("identity_retained_supersession_v1", &old.id)?, witness);
    let names = run_names(journal)?;
    check!(eq; names.iter().filter(|name| name.as_str() == "identity:rules").count(), 1);
    journal.drive(client, url).await?;
    check!(journal.held.is_some());
    Ok(())
}

fn lose_completion(journal: &mut Journal) -> TestResult {
    let run = journal
        .entries
        .iter()
        .find(|entry| {
            entry.kind == 0x0411
                && codec::field(&entry.payload, 12)
                    .is_ok_and(|name| name == Some(b"identity:rules".as_slice()))
        })
        .ok_or("deterministic Run absent")?;
    let completion_id = run.number(11)?;
    let position = journal
        .entries
        .iter()
        .position(|entry| {
            entry.kind == 0x8011 && entry.number(1).is_ok_and(|id| id == completion_id)
        })
        .ok_or("deterministic completion absent")?;
    journal.entries.truncate(position);
    Ok(())
}

async fn reach_rules(journal: &mut Journal, client: &reqwest::Client, url: &str) -> TestResult {
    for _ in 0..256 {
        journal.step(client, url).await?;
        if run_names(journal)?.contains(&"identity:rules".to_owned()) {
            return Ok(());
        }
    }
    Err("deterministic rules did not execute within the journal budget".into())
}

async fn historical(
    store: &Store,
    old: &ReviewCase,
    summary: &str,
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    let recorded = journal.entries.clone();
    let run = recorded
        .iter()
        .find(|entry| {
            entry.kind == 0x0411
                && codec::field(&entry.payload, 12)
                    .is_ok_and(|name| name == Some(b"identity:rules".as_slice()))
        })
        .ok_or("historical rules command absent")?;
    let completion = run.number(11)?;
    let result = recorded
        .iter()
        .find(|entry| entry.kind == 0x8011 && entry.number(1).is_ok_and(|id| id == completion))
        .ok_or("historical rules result absent")?;
    let value = codec::field(result.bytes(5)?, 1)?.ok_or("historical result value absent")?;
    check!(eq; value, serde_json::to_vec(summary)?.as_slice());
    check!(eq; store.derived_generation(), 0);
    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    let output = journal.step(client, url).await?;
    check!(!output.iter().any(|entry| {
        entry.kind == 0x0411
            && codec::field(&entry.payload, 12)
                .is_ok_and(|name| name == Some(b"identity:rules".as_slice()))
    }));
    check!(eq; journal.entries.get(..recorded.len()), Some(recorded.as_slice()));
    check!(eq; store.derived_generation(), 0);
    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    journal.drive(client, url).await?;
    check!(journal.held.is_some());
    check!(store.scan::<ReviewCase>(Table::ReviewCases)?.contains(old));
    check!(store
        .journal_payload("identity_retained_supersession_v1", &old.id)?
        .is_none());
    let names = run_names(journal)?;
    check!(eq; names, vec!["identity:rules", "identity:advice:0", "identity:advice:1", "identity:reconcile"]);
    Ok(())
}
