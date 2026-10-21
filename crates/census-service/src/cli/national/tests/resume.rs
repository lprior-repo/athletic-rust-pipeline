use super::*;

#[path = "resume/authority.rs"]
mod authority;

pub(super) async fn guarded_resume_ingress(
    listener: tokio::net::TcpListener,
    responses: Vec<(&str, String)>,
    completion: tokio::sync::oneshot::Receiver<()>,
) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    guarded_resume_with_held_reply(listener, responses, completion, None).await
}

pub(super) async fn guarded_resume_with_held_reply(
    listener: tokio::net::TcpListener,
    responses: Vec<(&str, String)>,
    mut completion: tokio::sync::oneshot::Receiver<()>,
    held_reply: Option<(usize, usize)>,
) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    let mut held_socket = None;
    let mut captured = Vec::new();
    let mut responses = responses.into_iter();
    loop {
        let socket = tokio::select! {
            biased;
            accepted = listener.accept() => accepted?.0,
            _ = &mut completion => { drop(held_socket); return Ok(captured); },
        };
        if captured.len() >= 1024 {
            return Err("resume fixture request count exceeds limit".into());
        }
        let (status, body) = match responses.next() {
            Some(response) => response,
            None => (
                "500 Internal Server Error",
                "{\"message\":\"unexpected request\"}".into(),
            ),
        };
        let declared = held_reply
            .filter(|(index, _)| *index == captured.len())
            .map(|(_, length)| length);
        let (record, held) = record_reply(socket, status, &body, declared).await?;
        if held.is_some() {
            held_socket = held;
        }
        captured.push(record);
    }
}

async fn record_reply(
    socket: tokio::net::TcpStream,
    status: &str,
    body: &str,
    held_length: Option<usize>,
) -> Result<((String, Vec<u8>), Option<tokio::net::TcpStream>), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    let mut socket = BufReader::new(socket);
    let mut path = String::new();
    socket.read_line(&mut path).await?;
    let mut length = 0;
    for _ in 0..128 {
        let mut header = String::new();
        socket.read_line(&mut header).await?;
        if header == "\r\n" {
            break;
        }
        if let Some((key, value)) = header.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>()?;
            }
        }
    }
    if length > 8192 {
        return Err("resume fixture request exceeds byte ceiling".into());
    }
    let mut payload = vec![0; length];
    socket.read_exact(&mut payload).await?;
    let invocation_header = if status.starts_with("470 ") {
        ""
    } else {
        "X-Restate-Id: inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5\r\n"
    };
    if held_length.is_some() && !body.is_empty() {
        return Err("held response must not provide a body".into());
    }
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{invocation_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        held_length.map_or(body.len(), |length| length)
    );
    socket.get_mut().write_all(response.as_bytes()).await?;
    if held_length.is_some() {
        return Ok(((path, payload), Some(socket.into_inner())));
    }
    socket.get_mut().shutdown().await?;
    Ok(((path, payload), None))
}

#[tokio::test]
async fn guarded_ingress_observes_unexpected_requests_even_when_errors_are_swallowed(
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let (finished, completion) = tokio::sync::oneshot::channel::<()>();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            guarded_resume_ingress(listener, vec![("200 OK", "{}".into())], completion),
            async {
                let first = client.post(format!("{origin}/expected")).send().await?;
                check!(eq; first.status(), reqwest::StatusCode::OK);
                let unexpected = client.post(format!("{origin}/unexpected")).send().await?;
                let swallowed_status = unexpected.status();
                drop(finished);
                Ok::<_, Box<dyn std::error::Error>>(swallowed_status)
            }
        )
    })
    .await?;
    let captured = captured?;
    check!(eq; outcome?, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
    check!(eq; captured.len(), 2);
    check!(captured
        .get(1)
        .is_some_and(|(path, _)| path == "POST /unexpected HTTP/1.1\r\n"));
    Ok(())
}

pub(super) fn bound_request(jurisdictions: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let store = "a".repeat(64);
    let profile = "b".repeat(64);
    let request: census_service::restate_services::NationalRequest = serde_json::from_value(
        serde_json::json!({
            "season": 2026, "revision": 1, "jurisdictions": jurisdictions,
            "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-01"},
            "publication_contract": "selected_publication_v2",
            "export_job": {"contract": "selected_publication_v1",
                "selection": {"version": 1, "store_identity": store, "profile_id": profile}},
            "pass_budget": 17,
            "pass_delay_seconds": 41, "concurrency": 7,
            "generation": "qualified-v1"
        }),
    )?;
    Ok(serde_json::to_string(
        &census_service::restate_services::NationalRequestBinding::Bound(Box::new(request)),
    )?)
}

pub(super) fn capabilities() -> Result<String, Box<dyn std::error::Error>> {
    Ok(serde_json::to_string(
        &census_service::restate_services::PublicationCapabilities::current(),
    )?)
}
