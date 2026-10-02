use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) fn local_url(host: &str, public_id: u64, page: u64) -> String {
    coach_url(page)
        .replacen("https://services.arbitersports.com", host, 1)
        .replace("EntityId=450", &format!("EntityId={public_id}"))
}

pub(super) fn local_fetcher(cache: &Path) -> Fetcher {
    Fetcher::new(
        cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        vec!["127.0.0.1".to_string()],
    )
    .expect("fixture fetcher")
}

pub(super) fn robots() -> (String, u16, Vec<u8>) {
    (
        "/robots.txt".to_string(),
        200,
        b"User-agent: *\r\nAllow: /\r\n".to_vec(),
    )
}

pub(super) fn reply(host: &str, url: &str, status: u16, body: &[u8]) -> (String, u16, Vec<u8>) {
    (
        url.strip_prefix(host).expect("fixture path").to_string(),
        status,
        body.to_vec(),
    )
}

pub(super) async fn serve_responses(
    listener: tokio::net::TcpListener,
    replies: Vec<(String, u16, Vec<u8>)>,
) {
    for (path, status, body) in replies {
        let (mut socket, _) = listener.accept().await.expect("fixture connection");
        let mut request = Vec::new();
        for _ in 0..16 {
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).await.expect("request bytes");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&bytes[..count]);
            if request.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let request = std::str::from_utf8(&request).expect("HTTP request UTF8");
        assert_eq!(request.split_whitespace().nth(1), Some(path.as_str()));
        let header = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket
            .write_all(header.as_bytes())
            .await
            .expect("fixture header");
        socket.write_all(&body).await.expect("fixture response");
    }
}

pub(super) fn capture_meta(cache: &Path, url: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(cache.join(format!("{}.meta.json", Fetcher::key_for("GET", url, ""))))
            .expect("capture metadata"),
    )
    .expect("capture JSON")
}

pub(super) fn assert_archived_capture(cache: &Path, body: &[u8], original_meta: &Value) {
    let digest = crate::net::cache::content_digest(body);
    let raw = cache.join("archive/bodies").join(format!("{digest}.body"));
    assert_eq!(std::fs::read(raw).expect("immutable original body"), body);
    let captures =
        std::fs::read_dir(cache.join("archive/captures").join(digest)).expect("capture metadata");
    let retained: Vec<Value> = captures
        .map(|entry| {
            serde_json::from_slice(
                &std::fs::read(entry.expect("capture entry").path()).expect("capture bytes"),
            )
            .expect("capture JSON")
        })
        .collect();
    assert!(retained.contains(original_meta));
}
