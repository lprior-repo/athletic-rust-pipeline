use futures::{stream::FuturesUnordered, StreamExt};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

#[tokio::test]
async fn ingress_http2_refused_stream_makes_exactly_one_physical_request() -> TestResult {
    let client = super::http_client_builder(REQUEST_TIMEOUT)
        .http2_prior_knowledge()
        .build()?;
    check!(eq; rejected_requests(client).await?, 1);
    Ok(())
}

#[tokio::test]
async fn http2_fixture_exercises_reqwests_default_protocol_nack_retries() -> TestResult {
    let client = reqwest::Client::builder()
        .no_proxy()
        .http2_prior_knowledge()
        .timeout(REQUEST_TIMEOUT)
        .build()?;
    check!(eq; rejected_requests(client).await?, 3);
    Ok(())
}

async fn rejected_requests(client: reqwest::Client) -> TestResult<usize> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!(
        "http://{}/restate/invocation/inv_fixture/output",
        listener.local_addr()?
    );
    let (finished, completion) = tokio::sync::oneshot::channel::<()>();
    let (count, response) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(serve(listener, completion), async {
            let response = client.get(url).send().await;
            drop(finished);
            response
        })
    })
    .await?;
    let error = response
        .err()
        .ok_or("REFUSED_STREAM returned an HTTP response")?;
    check!(error.is_request());
    check!(!error.is_timeout());
    check!(std::error::Error::source(&error).is_some());
    count
}

async fn serve(
    listener: TcpListener,
    mut completion: tokio::sync::oneshot::Receiver<()>,
) -> TestResult<usize> {
    let requests = Arc::new(AtomicUsize::new(0));
    let mut connections = FuturesUnordered::new();
    let mut accepted = 0usize;
    for _ in 0..32 {
        tokio::select! {
            biased;
            _ = &mut completion => return Ok(requests.load(Ordering::SeqCst)),
            socket = listener.accept() => {
                let (socket, _) = socket?;
                accepted += 1;
                if accepted > 4 {
                    return Err("H2 fixture connection ceiling exceeded".into());
                }
                connections.push(serve_connection(socket, Arc::clone(&requests)));
            }
            result = connections.next(), if !connections.is_empty() => {
                result.ok_or("H2 fixture connection result absent")??;
            }
        }
    }
    Err("H2 fixture event ceiling exceeded".into())
}

async fn serve_connection(mut socket: TcpStream, requests: Arc<AtomicUsize>) -> TestResult {
    let mut preface = [0u8; 24];
    socket.read_exact(&mut preface).await?;
    check!(eq; &preface, PREFACE);
    socket.write_all(&[0, 0, 0, 4, 0, 0, 0, 0, 0]).await?;
    for _ in 0..64 {
        let frame = match read_frame(&mut socket).await {
            Ok(frame) => frame,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
                ) =>
            {
                return Ok(())
            }
            Err(error) => return Err(error.into()),
        };
        match frame.kind {
            1 => {
                check!(frame.flags & 4 != 0);
                check!(frame.flags & 1 != 0);
                check!(frame.stream != [0, 0, 0, 0]);
                let [a, b, c, d] = frame.stream;
                socket
                    .write_all(&[0, 0, 4, 3, 0, a, b, c, d, 0, 0, 0, 7])
                    .await?;
                let observed = requests.fetch_add(1, Ordering::SeqCst);
                if observed >= 3 {
                    return Err("H2 fixture request ceiling exceeded".into());
                }
            }
            4 if frame.flags & 1 == 0 => {
                check!(eq; frame.stream, [0, 0, 0, 0]);
                socket.write_all(&[0, 0, 0, 4, 1, 0, 0, 0, 0]).await?;
            }
            4 | 8 | 3 | 7 => {}
            _ => return Err("unexpected H2 fixture frame".into()),
        }
    }
    Err("H2 fixture frame ceiling exceeded".into())
}

struct Frame {
    kind: u8,
    flags: u8,
    stream: [u8; 4],
}

async fn read_frame(socket: &mut TcpStream) -> std::io::Result<Frame> {
    let mut header = [0u8; 9];
    socket.read_exact(&mut header).await?;
    let [a, b, c, kind, flags, s0, s1, s2, s3] = header;
    let length = usize::try_from(u32::from_be_bytes([0, a, b, c]))
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let mut payload = [0u8; 16_384];
    let payload = payload.get_mut(..length).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "H2 fixture frame too large",
        )
    })?;
    socket.read_exact(payload).await?;
    Ok(Frame {
        kind,
        flags,
        stream: [s0 & 0x7f, s1, s2, s3],
    })
}
