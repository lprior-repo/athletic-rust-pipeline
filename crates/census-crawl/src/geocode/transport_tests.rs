use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::transport::{HttpTransport, Transport};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type DriveResult = Result<String, Box<dyn std::error::Error + Send + Sync>>;

const SECRET: &str = "super-secret-vendor-key";
const REQUEST_TIMEOUT_SECS: u64 = 10;

async fn read_request(socket: &mut tokio::net::TcpStream) -> DriveResult {
    let mut buffer = vec![0u8; 4096];
    let read = socket.read(&mut buffer).await?;
    Ok(buffer
        .get(..read)
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default())
}

fn respond(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn geocode_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/maps/api/geocode/json?address=100+Main+St&key={SECRET}")
}

async fn serve_once(
    listener: tokio::net::TcpListener,
    status: &'static str,
    body: &'static str,
) -> DriveResult {
    let (mut socket, _) = listener.accept().await?;
    let request = read_request(&mut socket).await?;
    socket.write_all(respond(status, body).as_bytes()).await?;
    Ok(request)
}

#[tokio::test]
async fn http_transport_sends_the_recorded_request_and_returns_the_body() -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let body = r#"{"status":"OK","results":[]}"#;
    let server = serve_once(listener, "200 OK", body);
    let call = async {
        let transport = HttpTransport::new()?;
        let reply = transport
            .get_json(
                geocode_url(address.port()).as_str(),
                Some("Bearer access-token"),
            )
            .await?;
        Ok::<String, Box<dyn std::error::Error + Send + Sync>>(reply)
    };

    let (request, reply) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(server, call)
    })
    .await??;

    assert_eq!(reply, body);
    assert!(
        request.starts_with("GET /maps/api/geocode/json?address=100+Main+St&key="),
        "{request}"
    );
    assert!(request.contains(SECRET), "{request}");
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer access-token"),
        "{request}"
    );
    Ok(())
}

#[tokio::test]
async fn a_vendor_error_status_becomes_a_failure_without_the_url_or_credential() -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = serve_once(listener, "500 Internal Server Error", "vendor unavailable");
    let call = async {
        let transport = HttpTransport::new()?;
        let failure = transport
            .get_json(geocode_url(address.port()).as_str(), None)
            .await
            .err()
            .ok_or("the vendor error status was accepted as a body")?;
        Ok::<String, Box<dyn std::error::Error + Send + Sync>>(failure.detail)
    };

    let (request, detail) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(server, call)
    })
    .await??;

    assert!(
        request.contains(SECRET),
        "the request never carried the key"
    );
    assert!(!detail.contains(SECRET), "{detail}");
    assert!(!detail.contains("127.0.0.1"), "{detail}");
    assert!(!detail.contains("maps/api"), "{detail}");
    assert!(!detail.is_empty(), "the failure carried no detail");
    Ok(())
}

#[tokio::test]
async fn http_transport_times_out_against_a_listener_that_never_answers() -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;

    let transport = HttpTransport::new()?;
    let started = std::time::Instant::now();
    let failure = transport
        .get_json(geocode_url(address.port()).as_str(), None)
        .await
        .err()
        .ok_or("the silent listener answered")?;
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_secs(REQUEST_TIMEOUT_SECS - 1),
        "the request failed after {elapsed:?}, before the {REQUEST_TIMEOUT_SECS} s request timeout"
    );
    assert!(!failure.detail.contains(SECRET), "{failure:?}");
    assert!(!failure.detail.contains("127.0.0.1"), "{failure:?}");
    assert!(!failure.detail.is_empty(), "{failure:?}");
    drop(listener);
    Ok(())
}

#[tokio::test]
async fn a_refused_connection_reports_without_the_url_or_credential() -> TestResult {
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        listener.local_addr()?.port()
    };

    let transport = HttpTransport::new()?;
    let failure = transport
        .get_json(geocode_url(port).as_str(), None)
        .await
        .err()
        .ok_or("a closed port answered")?;

    assert!(!failure.detail.contains(SECRET), "{failure:?}");
    assert!(!failure.detail.contains("127.0.0.1"), "{failure:?}");
    assert!(!failure.detail.contains("maps/api"), "{failure:?}");
    assert!(!failure.detail.is_empty(), "{failure:?}");
    Ok(())
}
