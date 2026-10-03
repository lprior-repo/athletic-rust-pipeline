use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::transport::{HttpTransport, Transport};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type DriveResult = Result<String, Box<dyn std::error::Error + Send + Sync>>;

const SECRET: &str = "super-secret-vendor-key";
const REQUEST_TIMEOUT_SECS: u64 = 10;

async fn read_request(socket: &mut tokio::net::TcpStream) -> DriveResult {
    let mut buffer = [0u8; 4096];
    let read = socket.read(&mut buffer).await?;
    let bytes = buffer
        .get(..read)
        .ok_or("request read exceeds bounded buffer")?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
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

#[test]
fn http_transport_sends_the_recorded_request_and_returns_the_body() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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

            check!(eq; reply, body);
            check!(
                request.starts_with("GET /maps/api/geocode/json?address=100+Main+St&key="),
                "{request}"
            );
            check!(request.contains(SECRET), "{request}");
            check!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer access-token"),
                "{request}"
            );
            Ok(())
        })
}

#[test]
fn a_vendor_error_status_becomes_a_failure_without_the_url_or_credential() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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

            check!(
                request.contains(SECRET),
                "the request never carried the key"
            );
            check!(!detail.contains(SECRET), "{detail}");
            check!(!detail.contains("127.0.0.1"), "{detail}");
            check!(!detail.contains("maps/api"), "{detail}");
            check!(!detail.is_empty(), "the failure carried no detail");
            Ok(())
        })
}

#[test]
fn http_transport_times_out_against_a_listener_that_never_answers() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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

            check!(
        elapsed >= Duration::from_secs(REQUEST_TIMEOUT_SECS - 1),
        "the request failed after {elapsed:?}, before the {REQUEST_TIMEOUT_SECS} s request timeout"
    );
            check!(!failure.detail.contains(SECRET), "{failure:?}");
            check!(!failure.detail.contains("127.0.0.1"), "{failure:?}");
            check!(!failure.detail.is_empty(), "{failure:?}");
            drop(listener);
            Ok(())
        })
}

#[test]
fn a_refused_connection_reports_without_the_url_or_credential() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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

            check!(!failure.detail.contains(SECRET), "{failure:?}");
            check!(!failure.detail.contains("127.0.0.1"), "{failure:?}");
            check!(!failure.detail.contains("maps/api"), "{failure:?}");
            check!(!failure.detail.is_empty(), "{failure:?}");
            Ok(())
        })
}
