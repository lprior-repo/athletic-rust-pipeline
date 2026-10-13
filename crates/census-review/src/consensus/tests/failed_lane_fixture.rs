use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use futures::channel::oneshot;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::super::support::TestResult;

const REQUEST_BYTES: usize = 1_100_000;

pub(super) enum Reply {
    Verdict(String),
    CloseAfterRequest,
}

pub(super) struct Lane {
    listener: TcpListener,
    reply: Reply,
    ready: oneshot::Sender<()>,
}

pub(super) async fn bind(reply: Reply) -> TestResult<(Lane, SocketAddr, oneshot::Receiver<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (ready, receiver) = oneshot::channel();
    Ok((
        Lane {
            listener,
            reply,
            ready,
        },
        address,
        receiver,
    ))
}

pub(super) async fn run<F, T>(lanes: [Lane; 2], operation: F) -> TestResult<(T, [Vec<Value>; 2])>
where
    F: Future<Output = TestResult<T>>,
{
    let [first, second] = lanes;
    let (output, first, second) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::try_join!(operation, first.serve(), second.serve())
    })
    .await??;
    Ok((output, [first, second]))
}

impl Lane {
    async fn serve(self) -> TestResult<Vec<Value>> {
        self.ready
            .send(())
            .map_err(|_| "lane readiness receiver dropped")?;
        let (mut stream, _) = self.listener.accept().await?;
        let request = read_request(&mut stream).await?;
        match self.reply {
            Reply::Verdict(content) => write_reply(&mut stream, &content).await?,
            Reply::CloseAfterRequest => {}
        }
        stream.shutdown().await?;
        drop(stream);
        Ok(vec![request])
    }
}

async fn write_reply(stream: &mut TcpStream, content: &str) -> TestResult {
    let body =
        serde_json::json!({ "choices": [{ "message": { "content": content } }] }).to_string();
    if body.len() > 262_144 {
        return Err("fixture response exceeds the model response bound".into());
    }
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_request(stream: &mut TcpStream) -> TestResult<Value> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    for _ in 0..REQUEST_BYTES {
        let count = stream.read(&mut buffer).await?;
        if count == 0 {
            return Err("request ended before its complete body arrived".into());
        }
        let length = request
            .len()
            .checked_add(count)
            .ok_or("request byte count overflow")?;
        if length > REQUEST_BYTES {
            return Err("fixture request exceeds its byte bound".into());
        }
        request.try_reserve(count)?;
        request.extend_from_slice(buffer.get(..count).ok_or("invalid request read count")?);
        if let Some(value) = parse_request(&request)? {
            return Ok(value);
        }
    }
    Err("fixture request exhausted its bounded read count".into())
}

fn parse_request(request: &[u8]) -> TestResult<Option<Value>> {
    let Some(head) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
        return Ok(None);
    };
    let body = head
        .checked_add(4)
        .ok_or("request header length overflow")?;
    let headers = std::str::from_utf8(request.get(..head).ok_or("missing request headers")?)?;
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>())
        })
        .ok_or("request body has no content length")??;
    let end = body
        .checked_add(length)
        .ok_or("request body length overflow")?;
    if end > REQUEST_BYTES {
        return Err("declared request length exceeds its byte bound".into());
    }
    let Some(bytes) = request.get(body..end) else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(bytes)?))
}
