use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) fn local_url(host: &str, public_id: u64, page: u64) -> String {
    coach_url(page)
        .replacen("https://services.arbitersports.com", host, 1)
        .replace("EntityId=450", &format!("EntityId={public_id}"))
}

pub(super) fn local_fetcher(cache: &Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        vec!["127.0.0.1".to_string()],
    )?)
}

pub(super) fn reply(
    host: &str,
    url: &str,
    status: u16,
    body: &[u8],
) -> TestResult<(String, u16, Vec<u8>)> {
    Ok((
        url.strip_prefix(host).ok_or("fixture path")?.to_string(),
        status,
        body.to_vec(),
    ))
}

pub(super) async fn serve_responses(
    listener: tokio::net::TcpListener,
    replies: Vec<(String, u16, Vec<u8>)>,
) -> TestResult<Vec<String>> {
    let mut accepted = Vec::new();
    for (path, status, body) in replies {
        let (mut socket, _) = listener.accept().await?;
        let mut request = Vec::new();
        for _ in 0..16 {
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).await?;
            if count == 0 {
                break;
            }
            request.extend_from_slice(&bytes[..count]);
            if request.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let request = std::str::from_utf8(&request)?;
        let requested = request.split_whitespace().nth(1).ok_or("request target")?;
        check!(eq; requested, path.as_str());
        accepted.push(requested.to_string());
        let header = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(header.as_bytes()).await?;
        socket.write_all(&body).await?;
    }
    Ok(accepted)
}

pub(super) async fn assert_request_conservation(
    fetcher: &Fetcher,
    accepted: &[String],
) -> TestResult {
    let count = u64::try_from(accepted.len())?;
    let stats = fetcher.stats().await;
    check!(eq; stats.physical_requests(), count);
    check!(eq;
        stats.per_host.get("127.0.0.1").ok_or("loopback traffic")?.physical_requests(),
        count
    );
    check!(eq; accepted.iter().filter(|path| path.as_str() == "/robots.txt").count(), 0);
    Ok(())
}

pub(super) fn capture_meta(cache: &Path, url: &str) -> TestResult<Value> {
    Ok(serde_json::from_slice(&std::fs::read(cache.join(
        format!("{}.meta.json", Fetcher::key_for("GET", url, "")),
    ))?)?)
}

pub(super) fn assert_archived_capture(
    cache: &Path,
    body: &[u8],
    original_meta: &Value,
) -> TestResult {
    let digest = crate::net::cache::content_digest(body);
    let raw = cache.join("archive/bodies").join(format!("{digest}.body"));
    check!(eq; std::fs::read(raw)?, body);
    let captures = std::fs::read_dir(cache.join("archive/captures").join(digest))?;
    let retained: Vec<Value> = captures
        .map(|entry| -> TestResult<Value> {
            Ok(serde_json::from_slice(&std::fs::read(entry?.path())?)?)
        })
        .collect::<TestResult<_>>()?;
    check!(retained.contains(original_meta));
    Ok(())
}
