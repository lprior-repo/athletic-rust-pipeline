use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn same_host_redirect_reaches_payload_but_unlisted_host_is_never_contacted() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = async move {
        let mut paths = Vec::new();
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await?;
            let mut bytes = [0; 4096];
            let length = socket.read(&mut bytes).await?;
            let request = std::str::from_utf8(bytes.get(..length).ok_or("request boundary")?)?;
            let path = request.split_whitespace().nth(1).ok_or("request path")?;
            paths.push(path.to_string());
            let response = match path {
                "/start" => "HTTP/1.1 302 Found\r\nLocation: /finish\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
                "/cross" => format!("HTTP/1.1 302 Found\r\nLocation: http://other.example:{}/hidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", address.port()),
                "/finish" => "HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nevidence".to_string(),
                other => return Err(format!("unapproved path contacted: {other}").into()),
            };
            socket.write_all(response.as_bytes()).await?;
        }
        check!(
            tokio::time::timeout(std::time::Duration::from_millis(30), listener.accept())
                .await
                .is_err()
        );
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(paths)
    };
    let guard = DestinationGuard::new(Vec::new());
    let client = reqwest::Client::builder()
        .no_proxy()
        .resolve("source.example", address)
        .resolve("other.example", address)
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            guard.redirect(attempt)
        }))
        .build()?;
    let requests = async {
        let url = format!("http://source.example:{}/start", address.port());
        check!(eq; client.get(url).send().await?.text().await?, "evidence");
        let url = format!("http://source.example:{}/cross", address.port());
        let error = client
            .get(url)
            .send()
            .await
            .err()
            .ok_or("unlisted redirect followed")?;
        check!(error.is_redirect());
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let (paths, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::try_join!(server, requests)
    })
    .await??;
    check!(eq; paths, ["/start", "/finish", "/cross"]);
    Ok(())
    })
}
