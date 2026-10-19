use super::{fixtures, journal};
use crate::restate_services::jobs::selected_fixture::Controlled;
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

pub(super) enum Scenario {
    Selected,
    MissingContractThenSelected,
}

pub(super) async fn exercise(scenario: Scenario) -> TestResult {
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
    let assertions = async {
        let result = async {
            let key = "selected-workbook-endpoint-fixture-v1";
            match scenario {
                Scenario::Selected => {
                    drive_selected_workbook(&controlled, &url, key, "selected-workbook-50", "50.00")
                        .await
                }
                Scenario::MissingContractThenSelected => {
                    refuse_omitted_workbook(&url, key).await?;
                    crate::restate_services::jobs::selected_fixture::revoke(&controlled)
                        .map_err(|error| error.to_string())?;
                    drive_selected_workbook(&controlled, &url, key, "selected-workbook-51", "51.00")
                        .await
                }
            }
        }
        .await;
        stop_tx
            .send(())
            .map_err(|_| "fixture Workbook endpoint lost its shutdown owner")?;
        result
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result
}

async fn drive_selected_workbook(
    controlled: &Controlled,
    url: &str,
    key: &str,
    invocation: &str,
    mark: &str,
) -> TestResult {
    tokio::time::timeout(Duration::from_secs(60), async {
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
        journal.invocation = invocation.to_owned();
        journal.identity = key.to_owned();
        journal.entries = vec![super::codec::Frame::new(
            0x0400,
            super::codec::blob(14, &super::codec::value(&payload)?)?,
        )];

        journal.drive(&client, url).await?;
        let output = journal
            .output
            .as_ref()
            .ok_or("Workbook endpoint did not complete the selected request")?;
        verify_selected_workbook_reply(controlled, output, mark)
    })
    .await?
}

fn verify_selected_workbook_reply(
    controlled: &Controlled,
    output: &serde_json::Value,
    mark: &str,
) -> TestResult {
    let reply: crate::restate_services::WorkbookReply = serde_json::from_value(output.clone())?;
    check!(eq; reply.grad_year, Some(2027));
    let publication =
        census_report::workbook::publication::verify_published(std::path::Path::new(&reply.path))?;
    check!(eq; publication.workbook.mapped_athletes, 1);
    crate::restate_services::jobs::publication_tests::check_selected_output(
        std::path::Path::new(&reply.path),
        controlled.public_id(),
        controlled.candidate_id(),
        controlled.current_school_id(),
        mark,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

async fn refuse_omitted_workbook(url: &str, key: &str) -> TestResult {
    tokio::time::timeout(Duration::from_secs(60), async {
        let client = reqwest::Client::builder().http2_prior_knowledge().build()?;
        let payload = serde_json::to_vec(&serde_json::json!({
            "grad_year": 2027,
            "scope": "all_sources",
            "school_year": 2026,
            "jurisdictions": &census_domain::UsJurisdiction::CENSUS_SCOPE[..],
        }))?;
        let mut journal = journal::Journal::new(&fixtures::request()?)?;
        journal.invocation = "omitted-workbook-contract".to_owned();
        journal.identity = key.to_owned();
        journal.entries = vec![super::codec::Frame::new(
            0x0400,
            super::codec::blob(14, &super::codec::value(&payload)?)?,
        )];
        let refused = journal.drive(&client, url).await;
        check!(refused.is_err());
        check!(eq; journal.output, None);
        Ok(())
    })
    .await?
}
