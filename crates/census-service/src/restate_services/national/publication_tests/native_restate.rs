mod endpoint;
mod node;
mod reaper;
use super::TestResult;
use endpoint::{call_workbook, read_output, register_endpoint};
use node::NativeNode;
use restate_sdk::prelude::{Endpoint, HttpServer};
use std::path::PathBuf;
use std::time::Duration;

const WORKFLOW_KEY: &str = "selected-workbook-native-e2e-v1";
const TEST_BUDGET: Duration = Duration::from_secs(240);

#[ignore = "explicit native Restate integration test; see docs/VERIFICATION-EVIDENCE.md"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_restate_restart_and_authority_revocation_preserve_selected_publication(
) -> TestResult {
    let controlled = crate::restate_services::jobs::selected_fixture::install()
        .map_err(|error| error.to_string())?;
    let mut node = NativeNode::new()?;
    node.start()?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(90))
        .build()?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let endpoint_address = listener.local_addr()?;
    let region = std::sync::Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(
        std::sync::Arc::new(controlled.store().clone()),
        std::sync::Arc::clone(&region),
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
    let server_task = tokio::spawn(server);
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
    let outcome = tokio::time::timeout(TEST_BUDGET, async {
        node.wait_ready(&client).await?;
        register_endpoint(&client, &node, endpoint_address).await?;
        let (first_status, invocation_id, first_body) =
            call_workbook(&client, &node.ingress, WORKFLOW_KEY, &request).await?;
        check!(eq; first_status, reqwest::StatusCode::OK,
            "first native Restate invocation failed: {}",
            String::from_utf8_lossy(&first_body));
        let reply: crate::restate_services::WorkbookReply = serde_json::from_slice(&first_body)?;
        check!(eq; reply.grad_year, Some(2027));
        let workbook_path = PathBuf::from(&reply.path);
        let publication = census_report::workbook::publication::verify_published(&workbook_path)?;
        check!(publication.workbook.mapped_athletes > 0);
        crate::restate_services::jobs::publication_tests::check_selected_output(
            &workbook_path,
            controlled.public_id(),
            controlled.candidate_id(),
            controlled.current_school_id(),
            "50.00",
        )
        .map_err(|error| error.to_string())?;
        let generation = workbook_path
            .parent()
            .ok_or("published generation is missing")?;
        let manifest_path = generation.join("manifest.json");
        let workbook_bytes = std::fs::read(&workbook_path)?;
        let manifest_bytes = std::fs::read(&manifest_path)?;
        let publication_root = controlled.store().out_dir().join("publication");
        let pointer_before = std::fs::read_link(publication_root.join("current"))?;
        check!(eq;
            census_report::workbook::publication::current_workbook(&publication_root)?,
            workbook_path
        );

        node.restart(&client).await?;
        let (retry_status, retry_body) =
            read_output(&client, &node.ingress, &invocation_id).await?;
        check!(eq; retry_status, reqwest::StatusCode::OK,
            "retry after native Restate restart failed: {}",
            String::from_utf8_lossy(&retry_body));
        check!(eq; retry_body, first_body, "Restate retry changed the exact reply bytes");

        crate::restate_services::jobs::selected_fixture::revoke(&controlled)
            .map_err(|error| error.to_string())?;
        node.stop().await?;
        let mut revoked_node = NativeNode::new()?;
        revoked_node.start()?;
        let refusal = tokio::time::timeout(TEST_BUDGET, async {
            revoked_node.wait_ready(&client).await?;
            register_endpoint(&client, &revoked_node, endpoint_address).await?;
            let (status, _invocation, body) =
                call_workbook(&client, &revoked_node.ingress, WORKFLOW_KEY, &request).await?;
            Ok::<_, Box<dyn std::error::Error>>((status, body))
        })
        .await;
        let revoked_node_stop = revoked_node.stop().await;
        let (refused_status, refused_body) = refusal??;
        revoked_node_stop?;
        check!(
            !refused_status.is_success(),
            "revoked selected authority unexpectedly published: {}",
            String::from_utf8_lossy(&refused_body)
        );
        check!(
            String::from_utf8_lossy(&refused_body)
                .contains("selected export job authority changed; retained input is preserved"),
            "native refusal did not identify the revoked authority: {}",
            String::from_utf8_lossy(&refused_body)
        );
        check!(eq; std::fs::read(&workbook_path)?, workbook_bytes);
        check!(eq; std::fs::read(&manifest_path)?, manifest_bytes);
        check!(eq; std::fs::read_link(publication_root.join("current"))?, pointer_before);
        check!(eq;
            census_report::workbook::publication::current_workbook(&publication_root)?,
            workbook_path
        );
        let verified_after_refusal =
            census_report::workbook::publication::verify_published(&workbook_path)?;
        check!(eq; verified_after_refusal.generation_digest, publication.generation_digest);
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;
    let node_stop = node.stop().await;
    let endpoint_stop = stop_tx.send(());
    let server_stop = tokio::time::timeout(Duration::from_secs(15), server_task).await;
    region.drain(Duration::from_secs(5)).await?;
    outcome??;
    node_stop?;
    endpoint_stop.map_err(|_| "Workbook endpoint lost its shutdown owner")?;
    server_stop.map_err(|_| "Workbook endpoint did not stop within 15 seconds")??;
    Ok(())
}
