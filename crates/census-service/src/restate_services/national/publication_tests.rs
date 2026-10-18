use super::NationalCensus;
use census_store::clock::Clock;
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

mod binding;
pub(crate) mod codec;
mod external;
mod fixtures;
pub(super) mod journal;
mod resume;
mod transport;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct FixtureClock;

impl Clock for FixtureClock {
    fn today(&self) -> String {
        "2026-10-12".to_owned()
    }

    fn today_iso8601(&self) -> String {
        "2026-10-12T00:00:00Z".to_owned()
    }

    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    ExternalControl,
    Address(binding::Scenario),
    Complete,
    ManagedComplete,
    ManagedFailedStage,
    SelectedFailedStage,
    ManagedRefusedSeal,
    ManagedCompletedResume,
    ManagedExhaustedResume,
    FailedStage(&'static str),
    RefusedSeal,
    PartialContacts,
    UnfinishedRosters,
    CompletedResume,
    ExhaustedResume,
    InvalidResume(&'static str),
}

#[tokio::test]
async fn current_fixture_exercises_external_replay_driver_without_historical_claim() -> TestResult {
    exercise(Scenario::ExternalControl).await
}

#[tokio::test]
async fn managed_national_publication_binds_one_original_request_and_one_bundle() -> TestResult {
    exercise(Scenario::ManagedComplete).await
}

#[tokio::test]
async fn managed_national_workbook_retry_preserves_the_same_call_and_request() -> TestResult {
    exercise(Scenario::ManagedFailedStage).await
}

#[tokio::test]
async fn selected_national_workbook_retry_preserves_the_same_profile_request() -> TestResult {
    exercise(Scenario::SelectedFailedStage).await
}

#[tokio::test]
async fn selected_workbook_endpoint_runs_the_reserved_handler_with_its_bound_profile() -> TestResult
{
    let controlled = crate::restate_services::jobs::selected_fixture::install()
        .map_err(|error| error.to_string())?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!("http://{}/invoke/Workbook/run", listener.local_addr()?);
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(
        Arc::new(controlled.store().clone()),
        Arc::clone(&region),
    );
    let endpoint = Endpoint::builder()
        .bind(crate::restate_services::Workbook::new(jobs))
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = tokio::time::timeout(Duration::from_secs(60), async {
            let client = reqwest::Client::builder()
                .http2_prior_knowledge()
                .timeout(Duration::from_secs(45))
                .build()?;
            let request = crate::restate_services::WorkbookRequest {
                grad_year: Some(2027),
                limit: None,
                scope: Some("all_sources".to_owned()),
                out: None,
                school_year: Some(2026),
                jurisdictions: census_domain::UsJurisdiction::CENSUS_SCOPE.to_vec(),
                export_job: census_report::export::ExportJobRequest::SelectedPublicationV1 {
                    selection: controlled.selection().map_err(|error| error.to_string())?,
                },
            };
            let payload = serde_json::to_vec(&request)?;
            let mut journal = journal::Journal::new(&fixtures::request()?)?;
            journal.invocation = "invocation-selected-workbook-endpoint".to_owned();
            journal.identity = "selected-workbook-endpoint-fixture-v1".to_owned();
            journal.entries = vec![codec::Frame::new(
                0x0400,
                codec::blob(14, &codec::value(&payload)?)?,
            )];

            journal.drive(&client, &url).await?;
            let output = journal
                .output
                .as_ref()
                .ok_or("Workbook endpoint did not complete the selected request")?;
            let reply: crate::restate_services::WorkbookReply =
                serde_json::from_value(output.clone())?;
            check!(eq; reply.grad_year, Some(2027));
            let publication = census_report::workbook::publication::verify_published(
                std::path::Path::new(&reply.path),
            )?;
            check!(publication.workbook.mapped_athletes > 0);
            crate::restate_services::jobs::publication_tests::check_selected_output(
                std::path::Path::new(&reply.path),
                controlled.public_id(),
                controlled.candidate_id(),
                controlled.current_school_id(),
                "50.00",
            )
            .map_err(|error| error.to_string())?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await;
        stop_tx
            .send(())
            .map_err(|_| "fixture Workbook endpoint lost its shutdown owner")?;
        result??;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result
}

#[tokio::test]
async fn managed_national_seal_refusal_never_rebuilds_its_bundle() -> TestResult {
    exercise(Scenario::ManagedRefusedSeal).await
}

#[tokio::test]
async fn managed_national_resume_preserves_source_keys_and_original_request() -> TestResult {
    exercise(Scenario::ManagedCompletedResume).await
}

#[tokio::test]
async fn managed_national_resume_cannot_reset_exhausted_source_attempts() -> TestResult {
    exercise(Scenario::ManagedExhaustedResume).await
}

#[tokio::test]
async fn completed_national_run_reaches_publication_without_manual_verb() -> TestResult {
    exercise(Scenario::Complete).await
}

#[tokio::test]
async fn failed_workbook_stage_resumes_the_existing_national_invocation() -> TestResult {
    exercise(Scenario::FailedStage("Workbook")).await
}

#[tokio::test]
async fn unready_seal_resumes_the_original_publication_call() -> TestResult {
    exercise(Scenario::FailedStage("Census")).await
}

#[tokio::test]
async fn refused_seal_refreshes_measurement_without_new_publication_jobs() -> TestResult {
    exercise(Scenario::RefusedSeal).await
}

#[tokio::test]
async fn incomplete_contacts_do_not_release_publication() -> TestResult {
    exercise(Scenario::PartialContacts).await
}

#[tokio::test]
async fn unfinished_jurisdiction_rosters_do_not_release_publication() -> TestResult {
    exercise(Scenario::UnfinishedRosters).await
}

#[tokio::test]
async fn completed_national_run_redrives_same_source_keys_and_replays_without_duplicate_calls(
) -> TestResult {
    exercise(Scenario::CompletedResume).await
}

#[tokio::test]
async fn national_resume_inherits_exhausted_source_attempts_and_never_reserves_attempt_four(
) -> TestResult {
    exercise(Scenario::ExhaustedResume).await
}

#[tokio::test]
async fn national_resume_rejects_noncanonical_or_unbounded_execution_keys() -> TestResult {
    for suffix in ["0", "01", "-1", "4294967296", "1:resume:2"] {
        exercise(Scenario::InvalidResume(suffix)).await?;
    }
    Ok(())
}
#[tokio::test]
async fn restricted_national_run_publishes_only_its_five_states() -> TestResult {
    restricted(crate::restate_services::PublicationContract::LegacySeparateV1).await
}

#[tokio::test]
async fn managed_national_bundle_preserves_the_selected_five_states() -> TestResult {
    restricted(crate::restate_services::PublicationContract::ManagedBundleV2).await
}

async fn restricted(contract: crate::restate_services::PublicationContract) -> TestResult {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let directory = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(directory.path())?);
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(store, Arc::clone(&region));
    let endpoint = Endpoint::builder()
        .bind(NationalCensus::new(Arc::new(FixtureClock), jobs))
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let client = reqwest::Client::builder()
                .http2_prior_knowledge()
                .timeout(Duration::from_secs(5))
                .build()?;
            let mut request = fixtures::request()?;
            request.publication_contract = contract;
            request.jurisdictions = vec![
                census_domain::UsJurisdiction::Michigan,
                census_domain::UsJurisdiction::Minnesota,
                census_domain::UsJurisdiction::Iowa,
                census_domain::UsJurisdiction::Illinois,
                census_domain::UsJurisdiction::Colorado,
            ];
            let mut journal = journal::Journal::new(&request)?;
            journal.drive(&client, &url).await?;
            let expected = serde_json::json!(["MI", "MN", "IA", "IL", "CO"]);
            let stages = if contract != crate::restate_services::PublicationContract::LegacySeparateV1 {
                vec!["Workbook"]
            } else {
                vec!["Report", "Bests", "Workbook"]
            };
            for stage in stages {
                let call = journal
                    .calls
                    .iter()
                    .find(|call| call.text(1).is_ok_and(|name| name == stage))
                    .ok_or("restricted publication stage absent")?;
                let body: serde_json::Value = serde_json::from_slice(call.bytes(3)?)?;
                check!(eq; body.get("jurisdictions"), Some(&expected));
            }
            let report = journal
                .output
                .as_ref()
                .ok_or("restricted national run did not return its report")?;
            check!(eq; report.get("jurisdictions").and_then(serde_json::Value::as_array).map(Vec::len), Some(5));
            Ok(())
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

async fn exercise(scenario: Scenario) -> TestResult {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let directory = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(directory.path())?);
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(store, Arc::clone(&region));
    let endpoint = Endpoint::builder()
        .bind(NationalCensus::new(Arc::new(FixtureClock), jobs))
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            resume::run_scenario(scenario, &url),
        )
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

async fn resume_stage(
    journal: &mut journal::Journal,
    client: &reqwest::Client,
    url: &str,
    stage: &str,
) -> TestResult {
    let held = journal
        .held
        .clone()
        .ok_or("national finished without awaiting its publication stage")?;
    check!(eq; held.text(1)?, stage);
    check!(eq; journal.output, None);
    let body = journal.request_body()?;
    let calls = journal.calls.clone();
    let retry = journal.step(client, url).await?;
    check!(eq; journal.request_body()?, body);
    check!(eq; journal.calls, calls);
    check!(retry.iter().any(|entry| entry.kind == 0x0001));
    check!(!retry.iter().any(|entry| entry.kind == 0x0003));
    journal.release()?;
    journal.drive(client, url).await?;
    check!(eq; journal.calls.iter().filter(|call| call == &&held).count(), 1);
    Ok(())
}

async fn refresh_seal(
    journal: &mut journal::Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    let first = journal
        .held
        .clone()
        .ok_or("initial seal measurement absent")?;
    journal.release_refusal()?;
    journal.drive(client, url).await?;
    let next = journal
        .held
        .as_ref()
        .ok_or("refusal failed to release a fresh measurement")?;
    check!(eq; first.text(6)?, format!("{}:seal:0", journal.identity));
    check!(eq; next.text(6)?, format!("{}:seal:1", journal.identity));
    check!(eq; next.bytes(3)?, first.bytes(3)?);
    let refusal = journal
        .entries
        .iter()
        .find(|entry| {
            entry.kind == 0x0403 && entry.text(1).is_ok_and(|key| key == "publication_refusal")
        })
        .ok_or("parent did not retain the observed terminal refusal")?;
    let bytes = codec::field(refusal.bytes(3)?, 1)?.ok_or("refusal state value missing")?;
    check!(eq; serde_json::from_slice::<String>(bytes)?, "the census seal refused: jurisdiction sweeps remain owed");
    resume_stage(journal, client, url, "Census").await?;
    Ok(())
}

fn publication_services(journal: &journal::Journal) -> TestResult<Vec<String>> {
    journal
        .calls
        .iter()
        .filter(|call| {
            call.text(1).is_ok_and(|name| {
                matches!(
                    name.as_str(),
                    "Consolidate" | "Report" | "Bests" | "Workbook"
                )
            }) || call.text(2).is_ok_and(|name| name == "publication")
        })
        .map(|call| call.text(1))
        .collect()
}

fn assert_published(journal: &journal::Journal) -> TestResult {
    let managed = journal.original.publication_contract
        != crate::restate_services::PublicationContract::LegacySeparateV1;
    let stages = if managed {
        vec!["Consolidate", "Workbook"]
    } else {
        vec!["Consolidate", "Report", "Bests", "Workbook"]
    };
    let mut expected = stages.clone();
    expected.push("Census");
    expected.extend((0..journal.seal_refusals).map(|_| "Census"));
    check!(eq; publication_services(journal)?, expected);
    binding::assert_requests(journal, managed)?;
    for stage in stages {
        let call = journal
            .calls
            .iter()
            .find(|call| call.text(1).is_ok_and(|name| name == stage))
            .ok_or("publication stage absent")?;
        check!(eq; journal.calls.iter().filter(|call| call.text(1).is_ok_and(|name| name == stage)).count(), 1);
        check!(eq; call.text(5)?, format!("{}:{}", journal.identity, stage.to_ascii_lowercase()));
    }
    if journal.original.publication_contract
        == crate::restate_services::PublicationContract::SelectedPublicationV1
    {
        let workbook = journal
            .calls
            .iter()
            .find(|call| call.text(1).is_ok_and(|name| name == "Workbook"))
            .ok_or("selected publication did not call Workbook")?;
        let body: serde_json::Value = serde_json::from_slice(workbook.bytes(3)?)?;
        check!(eq;
            body.get("export_job"),
            Some(&serde_json::to_value(&journal.original.export_job)?),
            "the immutable profile selection reaches the workbook handler"
        );
    }
    let seal = journal
        .calls
        .iter()
        .find(|call| call.text(2).is_ok_and(|handler| handler == "publication"))
        .ok_or("national never called the journal-measuring publication handler")?;
    check!(eq; seal.text(6)?, format!("{}:seal:0", journal.identity));
    let workbook = match journal.original.publication_contract {
        crate::restate_services::PublicationContract::SelectedPublicationV1 => journal
            .selected_workbook_path
            .as_deref()
            .ok_or("selected National dispatch did not run the controlled workbook job")?,
        crate::restate_services::PublicationContract::ManagedBundleV2 => fixtures::MANAGED_WORKBOOK,
        crate::restate_services::PublicationContract::LegacySeparateV1 => {
            "out/publication/current/workbook.xlsx"
        }
    };
    check!(eq; serde_json::from_slice::<serde_json::Value>(seal.bytes(3)?)?, serde_json::json!({
        "grad_year": 2027, "all_sources": true, "workbook": workbook,
        "write": true, "season": 2026, "revision": 1, "source_objects": [], "jurisdictions": journal.original.jurisdictions
    }));
    let report = journal
        .output
        .as_ref()
        .ok_or("national did not return its completed report")?;
    let count = census_reconcile::identity::admitted_scope(&journal.original.jurisdictions).len();
    check!(eq; report.get("jurisdictions").and_then(serde_json::Value::as_array).map(Vec::len), Some(count));
    check!(eq; report.get("failures"), Some(&serde_json::json!([])));
    check!(eq; report.get("owed"), Some(&serde_json::json!([])));
    if journal.original.publication_contract
        == crate::restate_services::PublicationContract::SelectedPublicationV1
    {
        let controlled = journal
            .selected_workbook
            .as_ref()
            .ok_or("selected National controlled fixture missing")?;
        let path = journal
            .selected_workbook_path
            .as_ref()
            .ok_or("selected National workbook output missing")?;
        let verified =
            census_report::workbook::publication::verify_published(std::path::Path::new(path))?;
        check!(verified.workbook.mapped_athletes > 0);
        crate::restate_services::jobs::publication_tests::check_selected_output(
            std::path::Path::new(path),
            controlled.public_id(),
            controlled.candidate_id(),
            controlled.current_school_id(),
            "50.00",
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}
