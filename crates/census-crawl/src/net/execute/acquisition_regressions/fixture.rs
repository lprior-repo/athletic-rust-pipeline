use crate::net::{FetchOptions, Fetcher};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(super) fn fetcher(root: &std::path::Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        root.join("http"),
        None,
        Duration::from_millis(1),
        HashMap::new(),
        vec!["127.0.0.1".into()],
    )?)
}

async fn read_request(socket: &mut TcpStream) -> TestResult<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    for _ in 0..16 {
        let count = socket.read(&mut buffer).await?;
        if count == 0 {
            return Err("incomplete request".into());
        }
        bytes.extend_from_slice(buffer.get(..count).ok_or("read length")?);
        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
            return Ok(String::from_utf8(bytes)?);
        }
    }
    Err("request exceeded fixture bound".into())
}

pub(super) async fn serve(listener: TcpListener, count: usize) -> TestResult<Vec<String>> {
    let mut requests = Vec::new();
    for _ in 0..count {
        let (mut socket, _) = listener.accept().await?;
        let request = read_request(&mut socket).await?;
        let lower = request.to_ascii_lowercase();
        let (status, headers, body) = response_for(&request, &lower);
        let response = format!(
            "HTTP/1.1 {status} Fixture\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await?;
        requests.push(request);
    }
    Ok(requests)
}

fn response_for(request: &str, lower: &str) -> (u16, &'static str, &'static str) {
    if request.starts_with("GET /robots.txt ") {
        (200, "", "User-agent: *\r\nAllow: /\r\n")
    } else if request.starts_with("GET /redirect ") {
        (302, "Location: /payload\r\n", "")
    } else if request.starts_with("GET /limited ") {
        (429, "Retry-After: 1\r\n", "slow down")
    } else if lower.contains("if-none-match:") || lower.contains("if-modified-since:") {
        (304, "", "")
    } else if lower.contains("accept-language: fr") {
        (200, "Vary: Accept, Accept-Language\r\n", "bonjour")
    } else if lower.contains("accept: application/json") {
        (200, "Vary: Accept, Accept-Language\r\n", "{\"value\":1}")
    } else {
        (200, "Vary: Accept, Accept-Language\r\n", "public html")
    }
}

pub(super) fn options(headers: &[(&str, &str)]) -> FetchOptions {
    FetchOptions {
        headers: headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
        ..FetchOptions::default()
    }
}

pub(super) async fn independent_representations(
    headers: [FetchOptions; 3],
    bodies: [&[u8]; 2],
) -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/payload", listener.local_addr()?);
    let client = async {
        let [first_options, second_options, equivalent_options] = headers;
        let first = fetcher.get(&url, &first_options).await?;
        let second = fetcher.get(&url, &second_options).await?;
        check!(eq; first.body.as_slice(), bodies[0]);
        check!(eq; second.body.as_slice(), bodies[1]);
        check!(!first.from_cache);
        check!(!second.from_cache);
        for (request, expected) in [(&equivalent_options, &first), (&second_options, &second)] {
            let replay = fetcher.get(&url, request).await?;
            check!(replay.from_cache);
            check!(eq; replay.body, expected.body);
            check!(eq; replay.fetched_at, expected.fetched_at);
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let (requests, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::try_join!(serve(listener, 3), client)
    })
    .await??;
    check!(eq; requests.len(), 3);
    Ok(())
}
