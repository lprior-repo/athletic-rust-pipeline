use super::TestResult;
use bytes::Bytes;
use http::{Method, Request};
use std::future::Future;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub(super) enum WireBody {
    Bytes(Vec<u8>),
    Undeclared(Vec<u8>),
    Chunked(Vec<Vec<u8>>),
    Hold,
    Repeated {
        prefix: &'static [u8],
        count: usize,
        suffix: &'static [u8],
    },
}

pub(super) struct Reply {
    status: &'static str,
    headers: Vec<(String, String)>,
    body: WireBody,
    expected_peer_close: bool,
}

impl Reply {
    pub(super) fn new(status: &'static str, body: WireBody) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body,
            expected_peer_close: false,
        }
    }
    pub(super) fn refusing_body(mut self) -> Self {
        self.expected_peer_close = true;
        self
    }
    pub(super) fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

pub(super) struct Captured {
    pub(super) path: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

pub(super) fn request(origin: &str, path: &str, method: Method) -> TestResult<Request<Bytes>> {
    Ok(Request::builder()
        .uri(format!("{origin}{path}"))
        .method(method)
        .body(Bytes::new())?)
}

pub(super) async fn exchange<F, Fut, T>(reply: Reply, action: F) -> TestResult<(Captured, T)>
where
    F: FnOnce(String, reqwest::Client) -> Fut,
    Fut: Future<Output = T>,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let transport = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let (finished, mut completion) = tokio::sync::oneshot::channel::<()>();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        tokio::join!(async {
            let (socket, _) = listener.accept().await?;
            let mut socket = BufReader::new(socket);
            let captured = read_request(&mut socket).await?;
            respond(socket.get_mut(), &reply).await?;
            tokio::select! {
                biased;
                accepted = listener.accept() => { let _ = accepted?; Err("transport unexpectedly submitted another request".into()) },
                _ = &mut completion => Ok::<_, Box<dyn std::error::Error>>(captured),
            }
        }, async {
            let result = action(origin, transport).await;
            drop(finished);
            result
        })
    }).await?;
    Ok((captured?, outcome))
}

async fn read_request(socket: &mut BufReader<tokio::net::TcpStream>) -> TestResult<Captured> {
    let mut path = String::new();
    socket.read_line(&mut path).await?;
    let mut length = 0;
    let mut headers = Vec::new();
    for _ in 0..128 {
        let mut header = String::new();
        socket.read_line(&mut header).await?;
        if header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            let value = value.trim();
            if name.eq_ignore_ascii_case("content-length") {
                length = value.parse::<usize>()?;
            }
            headers.push((name.into(), value.into()));
        }
    }
    if length > 16384 {
        return Err("request fixture byte limit exceeded".into());
    }
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await?;
    Ok(Captured {
        path,
        headers,
        body,
    })
}

async fn respond(socket: &mut tokio::net::TcpStream, reply: &Reply) -> TestResult {
    let outcome = async {
        write_response(socket, reply).await?;
        if !matches!(reply.body, WireBody::Hold) {
            socket.shutdown().await?;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    outcome.or_else(|error| {
        let expected = reply.expected_peer_close
            && error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                )
            });
        if expected {
            Ok(())
        } else {
            Err(error)
        }
    })
}

async fn write_response(socket: &mut tokio::net::TcpStream, reply: &Reply) -> TestResult {
    let mut head = format!(
        "HTTP/1.1 {}\r\nConnection: close\r\nContent-Type: application/json\r\n",
        reply.status
    );
    for (name, value) in &reply.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    match &reply.body {
        WireBody::Bytes(body) => head.push_str(&format!("Content-Length: {}\r\n", body.len())),
        WireBody::Repeated {
            prefix,
            count,
            suffix,
        } => head.push_str(&format!(
            "Content-Length: {}\r\n",
            prefix.len() + count + suffix.len()
        )),
        WireBody::Chunked(_) => head.push_str("Transfer-Encoding: chunked\r\n"),
        WireBody::Undeclared(_) | WireBody::Hold => {}
    }
    head.push_str("\r\n");
    socket.write_all(head.as_bytes()).await?;
    match &reply.body {
        WireBody::Bytes(body) | WireBody::Undeclared(body) => socket.write_all(body).await?,
        WireBody::Hold => {}
        WireBody::Chunked(parts) => {
            for part in parts {
                socket
                    .write_all(format!("{:x}\r\n", part.len()).as_bytes())
                    .await?;
                socket.write_all(part).await?;
                socket.write_all(b"\r\n").await?;
            }
            socket.write_all(b"0\r\n\r\n").await?;
        }
        WireBody::Repeated {
            prefix,
            count,
            suffix,
        } => {
            socket.write_all(prefix).await?;
            let block = [b'A'; 65536];
            let mut remaining = *count;
            while remaining > 0 {
                let size = remaining.min(block.len());
                socket
                    .write_all(block.get(..size).ok_or("fixture block range")?)
                    .await?;
                remaining -= size;
            }
            socket.write_all(suffix).await?;
        }
    }
    Ok(())
}
