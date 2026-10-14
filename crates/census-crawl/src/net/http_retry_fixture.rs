use std::future::Future;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const PREFACE: &[u8; 24] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const SETTINGS: &[u8; 9] = &[0, 0, 0, 4, 0, 0, 0, 0, 0];
const SETTINGS_ACK: &[u8; 9] = &[0, 0, 0, 4, 1, 0, 0, 0, 0];
const MAX_FRAME_BYTES: usize = 16_384;
const MAX_FRAMES: usize = 32;

struct Frame {
    kind: u8,
    flags: u8,
    stream: u32,
    payload: Vec<u8>,
}

async fn read_frame(socket: &mut TcpStream) -> TestResult<Frame> {
    let mut header = [0_u8; 9];
    socket.read_exact(&mut header).await?;
    let [a, b, c, kind, flags, d, e, f, g] = header;
    let length = usize::try_from(u32::from_be_bytes([0, a, b, c]))?;
    if length > MAX_FRAME_BYTES {
        return Err(format!("fixture frame exceeded {MAX_FRAME_BYTES} bytes: {length}").into());
    }
    let mut payload = vec![0_u8; length];
    socket.read_exact(&mut payload).await?;
    Ok(Frame {
        kind,
        flags,
        stream: u32::from_be_bytes([d, e, f, g]) & 0x7fff_ffff,
        payload,
    })
}

async fn refuse_frame(socket: &mut TcpStream, frame: Frame, requests: &mut Vec<u32>) -> TestResult {
    match frame.kind {
        1 => {
            check!(frame.stream > 0 && frame.stream % 2 == 1);
            check!(eq; frame.flags & 0x5, 0x5);
            check!(!frame.payload.is_empty());
            requests.push(frame.stream);
            let [a, b, c, d] = frame.stream.to_be_bytes();
            socket
                .write_all(&[0, 0, 4, 3, 0, a, b, c, d, 0, 0, 0, 7])
                .await?;
        }
        4 => {
            check!(eq; frame.stream, 0);
            match frame.flags {
                0 => {
                    check!(eq; frame.payload.len() % 6, 0);
                    socket.write_all(SETTINGS_ACK).await?;
                }
                1 => check!(eq; frame.payload.len(), 0),
                flags => return Err(format!("unexpected SETTINGS flags {flags}").into()),
            }
        }
        8 => check!(eq; frame.payload.len(), 4),
        kind => return Err(format!("unexpected fixture frame kind {kind}").into()),
    }
    Ok(())
}

async fn serve_refused_streams(
    listener: TcpListener,
    mut completed: oneshot::Receiver<()>,
) -> TestResult<Vec<u32>> {
    let (mut socket, _) = listener.accept().await?;
    let mut preface = [0_u8; 24];
    socket.read_exact(&mut preface).await?;
    check!(eq; &preface, PREFACE);
    socket.write_all(SETTINGS).await?;
    let mut requests = Vec::new();
    for _ in 0..MAX_FRAMES {
        let frame = tokio::select! {
            biased;
            frame = read_frame(&mut socket) => frame?,
            completed = &mut completed => {
                completed?;
                return Ok(requests);
            }
        };
        refuse_frame(&mut socket, frame, &mut requests).await?;
    }
    Err(format!("fixture exceeded {MAX_FRAMES} frames").into())
}

pub(crate) async fn observe_refusals<F>(
    listener: TcpListener,
    acquisition: F,
) -> TestResult<(Vec<u32>, F::Output)>
where
    F: Future,
{
    let (finished, completed) = oneshot::channel();
    let request = async {
        let result = acquisition.await;
        finished
            .send(())
            .map_err(|()| "fixture ended before acquisition completed")?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(result)
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        tokio::try_join!(serve_refused_streams(listener, completed), request)
    })
    .await?
}

pub(crate) fn check_refused_stream(error: &reqwest::Error) -> TestResult {
    check!(
        format!("{error:?}").contains("REFUSED_STREAM"),
        "expected a remote REFUSED_STREAM transport error: {error:?}"
    );
    Ok(())
}
