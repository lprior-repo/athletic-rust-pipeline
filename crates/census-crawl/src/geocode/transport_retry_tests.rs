use super::{client_builder, HttpTransport, Transport};
use crate::http_retry_fixture::{observe_refusals, TestResult};
use tokio::net::TcpListener;

#[tokio::test]
async fn geocode_transport_does_not_retry_refused_stream() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!(
        "http://{}/maps/api/geocode/json?address=100+Main+St&key=fixture-vendor-key",
        listener.local_addr()?
    );
    let mut transport = HttpTransport::new()?;
    transport.client = client_builder()
        .no_proxy()
        .http2_prior_knowledge()
        .build()?;
    let (requests, outcome) = observe_refusals(
        listener,
        transport.get_json(&url, Some("Bearer fixture-postal-token")),
    )
    .await?;
    check!(eq; requests, [1]);
    let failure = outcome
        .err()
        .ok_or("geocode transport unexpectedly succeeded")?;
    check!(eq; failure.detail, "error sending request");
    check!(!failure.detail.contains("fixture-vendor-key"));
    check!(!failure.detail.contains("fixture-postal-token"));
    check!(!failure.detail.contains("127.0.0.1"));
    check!(!failure.detail.contains("maps/api"));
    Ok(())
}
