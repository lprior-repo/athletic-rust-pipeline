use super::*;

#[tokio::test]
async fn serialized_full_browser_body_above_32_mib_is_admitted_without_changing_wire_data(
) -> TestResult {
    let reply = Reply::new("200 OK", WireBody::Repeated {
        prefix: br#"{"outcome":"captured","response":{"status":200,"headers":[],"body":""#,
        count: 44_739_243,
        suffix: br#"=","rankings":null},"challenge":false,"retry_after_ms":null,"fetched_at_ms":null}"#,
    }).header("x-restate-id", ID);
    let (captured, outcome) = exchange(reply, |origin, transport| async move {
        let client = client(origin.parse()?, transport)?;
        let response = client
            .request::<Json<serde_json::Value>, Json<crate::net::bridge::BrowserOutcome>>(
                RequestTarget::object("BrowserSession", "profile-0", "fetch"),
                Json(serde_json::json!({"original": true})),
            )
            .call()
            .await?
            .into_body()?
            .0;
        let crate::net::bridge::BrowserOutcome::Captured(capture) = response else {
            return Err("browser capture changed outcome".into());
        };
        check!(eq; capture.response.body.len(), 44_739_244);
        check!(capture.response.body.ends_with('='));
        check!(capture.response.body.len() > 33_554_432);
        check!(eq; capture.response.status, 200);
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await?;
    outcome?;
    check!(eq; captured.path, "POST /BrowserSession/profile-0/fetch HTTP/1.1\r\n");
    Ok(())
}
