use super::fixture::{TestResult, CERT, HOST, KEY};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{rustls, TlsAcceptor};

pub(super) struct Reply<'a> {
    pub(super) path: &'a str,
    pub(super) status: u16,
    pub(super) body: &'a [u8],
    pub(super) content_type: &'a str,
    pub(super) headers: &'a str,
}

impl<'a> Reply<'a> {
    pub(super) fn directory(body: &'a [u8]) -> Self {
        Self {
            path: "/teams",
            status: 200,
            body,
            content_type: "text/html",
            headers: "",
        }
    }
    pub(super) fn redirect(headers: &'a str) -> Self {
        Self {
            path: "/teams",
            status: 302,
            body: b"",
            content_type: "text/html",
            headers,
        }
    }
}

pub(super) async fn serve<const N: usize>(
    listener: TcpListener,
    replies: [Reply<'_>; N],
) -> TestResult<Vec<String>> {
    check!(N <= 8);
    let acceptor = acceptor()?;
    let mut requests = Vec::with_capacity(N);
    for reply in replies {
        let (socket, _) =
            tokio::time::timeout(Duration::from_secs(10), listener.accept()).await??;
        let mut stream =
            tokio::time::timeout(Duration::from_secs(10), acceptor.accept(socket)).await??;
        let request =
            tokio::time::timeout(Duration::from_secs(10), read_request(&mut stream)).await??;
        verify_request(&request, reply.path)?;
        requests.push(request);
        let header = format!(
            "HTTP/1.1 {} Controlled\r\nContent-Length: {}\r\nContent-Type: {}\r\nETag: \"public-root-v1\"\r\nX-Capture: controlled\r\n{}Connection: close\r\n\r\n",
            reply.status, reply.body.len(), reply.content_type, reply.headers,
        );
        tokio::time::timeout(Duration::from_secs(10), async {
            stream.write_all(header.as_bytes()).await?;
            stream.write_all(reply.body).await?;
            stream.shutdown().await
        })
        .await??;
    }
    Ok(requests)
}

fn acceptor() -> TestResult<TlsAcceptor> {
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(CERT.to_vec())],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec())),
    )?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

async fn read_request(stream: &mut (impl tokio::io::AsyncRead + Unpin)) -> TestResult<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    for _ in 0..64 {
        let count = stream.read(&mut buffer).await?;
        if count == 0 {
            return Err("controlled request ended before headers".into());
        }
        bytes.extend_from_slice(buffer.get(..count).ok_or("controlled request read bound")?);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(String::from_utf8(bytes)?);
        }
    }
    Err("controlled request exceeded 64 KiB".into())
}

fn verify_request(request: &str, path: &str) -> TestResult {
    check!(eq; request.lines().next(), Some(format!("GET {path} HTTP/1.1").as_str()));
    let lower = request.to_ascii_lowercase();
    for header in [
        format!("host: {HOST}"),
        "accept: */*".into(),
        "accept-encoding: identity".into(),
        "user-agent: census-service/0.1".into(),
    ] {
        check!(eq; lower.lines().filter(|line| *line == header).count(), 1);
    }
    Ok(())
}
