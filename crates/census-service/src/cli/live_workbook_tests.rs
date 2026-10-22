use super::*;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Captured = (String, Vec<u8>);

const ID: &str = "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5";
const REPLY: &str = "{\"path\":\"/original/workbook.xlsx\",\"grad_year\":2027}";
const INVOCATION_HEADER: &str = "x-restate-id: inv_ExistingWorkbook123\r\n";

fn request() -> TestResult<WorkbookRequest> {
    let selection = census_domain::model::ProfileSelectionV1::new(
        census_domain::model::StoreIdentity::parse(&"a".repeat(64))?,
        census_domain::model::ProfileId::parse(&"b".repeat(64))?,
    );
    Ok(WorkbookRequest {
        grad_year: Some(2027),
        limit: Some(50),
        scope: Some("core".to_string()),
        out: Some("/tmp/workbook-origin".to_string()),
        school_year: Some(2026),
        jurisdictions: Vec::new(),
        export_job: census_report::export::ExportJobRequest::SelectedPublicationV1 { selection },
    })
}

fn key() -> TestResult<String> {
    Ok(workbook_request_key(
        &request()?,
        &ExportGeneration::parse("03")?,
    )?)
}

fn response(status: &str, headers: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn resolved() -> String {
    response(
        "200 OK",
        "",
        &serde_json::json!({"invocationId": ID}).to_string(),
    )
}

async fn read_request(socket: &mut BufReader<tokio::net::TcpStream>) -> TestResult<Captured> {
    let mut path = String::new();
    socket.read_line(&mut path).await?;
    let mut length = 0;
    let mut complete = false;
    for _ in 0..128 {
        let mut header = String::new();
        socket.read_line(&mut header).await?;
        if header == "\r\n" {
            complete = true;
            break;
        }
        if let Some((key, value)) = header.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>()?;
            }
        }
    }
    if !complete || length > 16384 {
        return Err("workbook fixture request exceeds its bounds".into());
    }
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await?;
    Ok((path, body))
}

async fn serve(
    listener: tokio::net::TcpListener,
    responses: Vec<String>,
    mut completion: tokio::sync::oneshot::Receiver<()>,
) -> TestResult<Vec<Captured>> {
    if responses.len() > 3 {
        return Err("workbook fixture response count exceeds its bound".into());
    }
    let mut captured = Vec::new();
    loop {
        let socket = tokio::select! {
            biased;
            accepted = listener.accept() => accepted?.0,
            _ = &mut completion => return Ok(captured),
        };
        let response = responses
            .get(captured.len())
            .ok_or("unexpected workbook request")?;
        let mut socket = BufReader::new(socket);
        captured.push(read_request(&mut socket).await?);
        socket.get_mut().write_all(response.as_bytes()).await?;
        socket.get_mut().shutdown().await?;
    }
}

async fn exchange(
    responses: Vec<String>,
) -> TestResult<(Vec<Captured>, anyhow::Result<WorkbookReply>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let request = request()?;
    let generation = ExportGeneration::parse("03")?;
    let (finished, completion) = tokio::sync::oneshot::channel::<()>();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(serve(listener, responses, completion), async {
            let outcome = workbook(Some(&origin), request, &generation).await;
            drop(finished);
            outcome
        })
    })
    .await?;
    Ok((captured?, outcome))
}

fn assert_call(captured: &[Captured]) -> TestResult {
    let (path, body) = captured.first().ok_or("workbook call absent")?;
    let encoded = key()?.replace(':', "%3A").replace('/', "%2F");
    check!(eq; path, &format!("POST /Workbook/{encoded}/run HTTP/1.1\r\n"));
    check!(eq; serde_json::from_slice::<serde_json::Value>(body)?, serde_json::to_value(request()?)?);
    Ok(())
}

fn assert_lookup(captured: &[Captured]) -> TestResult {
    assert_call(captured)?;
    let (path, body) = captured.get(1).ok_or("workbook lookup absent")?;
    check!(eq; path, "POST /restate/lookup HTTP/1.1\r\n");
    check!(eq; serde_json::from_slice::<serde_json::Value>(body)?, serde_json::json!({
        "target": "workflow", "workflowName": "Workbook", "workflowKey": key()?
    }));
    Ok(())
}

fn assert_attach(captured: &[Captured]) -> TestResult {
    assert_lookup(captured)?;
    let (path, body) = captured.get(2).ok_or("workbook attachment absent")?;
    check!(eq; path, &format!("GET /restate/invocation/{ID}/attach HTTP/1.1\r\n"));
    check!(body.is_empty(), "attachment must not resubmit the request");
    Ok(())
}

fn assert_reply(reply: WorkbookReply) -> TestResult {
    check!(eq; reply.path, "/original/workbook.xlsx");
    check!(eq; reply.grad_year, Some(2027));
    Ok(())
}

#[tokio::test]
async fn workbook_completed_call_returns_original_reply_without_lookup() -> TestResult {
    let (captured, outcome) = exchange(vec![response("200 OK", INVOCATION_HEADER, REPLY)]).await?;
    check!(eq; captured.len(), 1);
    assert_call(&captured)?;
    assert_reply(outcome?)
}

#[tokio::test]
async fn workbook_conflict_reattaches_exact_workflow_with_or_without_invocation_header(
) -> TestResult {
    for headers in ["", INVOCATION_HEADER] {
        let (captured, outcome) = exchange(vec![
            response(
                "409 Conflict",
                headers,
                "{\"message\":\"workflow method already invoked\"}",
            ),
            resolved(),
            response("200 OK", "", REPLY),
        ])
        .await?;
        check!(eq; captured.len(), 3);
        assert_attach(&captured)?;
        assert_reply(outcome?)?;
    }
    Ok(())
}

#[tokio::test]
async fn workbook_other_status_errors_preserve_message_without_lookup() -> TestResult {
    for status in [
        "400 Bad Request",
        "404 Not Found",
        "500 Internal Server Error",
        "302 Found",
    ] {
        for headers in ["", INVOCATION_HEADER] {
            let (captured, outcome) = exchange(vec![response(
                status,
                headers,
                "{\"message\":\"original workbook failure\"}",
            )])
            .await?;
            check!(eq; captured.len(), 1);
            assert_call(&captured)?;
            let error = outcome.err().ok_or("non-conflict status accepted")?;
            let error = format!("{error:#}");
            check!(error.contains(status), "{error}");
            check!(error.contains("original workbook failure"), "{error}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn workbook_malformed_success_refuses_without_lookup() -> TestResult {
    let (captured, outcome) =
        exchange(vec![response("200 OK", INVOCATION_HEADER, "not json")]).await?;
    check!(eq; captured.len(), 1);
    assert_call(&captured)?;
    check!(outcome.is_err(), "malformed successful output accepted");
    Ok(())
}

#[tokio::test]
async fn workbook_lookup_refusals_do_not_attach_or_resubmit() -> TestResult {
    let malformed = serde_json::json!({"invocationId": ID, "workflowKey": "foreign"}).to_string();
    for (status, body) in [
        ("200 OK", "{}"),
        ("200 OK", "not json"),
        ("200 OK", "{\"invocationId\":\"inv_abc/foreign\"}"),
        ("200 OK", malformed.as_str()),
        ("404 Not Found", "{}"),
    ] {
        let (captured, outcome) = exchange(vec![
            response("409 Conflict", "", "{}"),
            response(status, "", body),
        ])
        .await?;
        check!(eq; captured.len(), 2);
        assert_lookup(&captured)?;
        check!(outcome.is_err(), "invalid lookup accepted: {status} {body}");
    }
    Ok(())
}

#[tokio::test]
async fn workbook_attachment_refusals_do_not_resubmit() -> TestResult {
    for (status, body) in [
        (
            "404 Not Found",
            "{\"message\":\"original attachment failure\"}",
        ),
        ("200 OK", "not json"),
    ] {
        let (captured, outcome) = exchange(vec![
            response("409 Conflict", "", "{}"),
            resolved(),
            response(status, "", body),
        ])
        .await?;
        check!(eq; captured.len(), 3);
        assert_attach(&captured)?;
        let error = outcome.err().ok_or("invalid attachment accepted")?;
        if status == "404 Not Found" {
            check!(
                format!("{error:#}").contains("original attachment failure"),
                "{error:#}"
            );
        }
    }
    Ok(())
}
