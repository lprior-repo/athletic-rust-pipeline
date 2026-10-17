use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use url::Url;

pub(super) const BODY: &[u8] = b"<!doctype html><html><head><title>native publication fixture</title></head><body><p>fixture</p></body></html>";
const MAX_REQUESTS: usize = 64;
const MAX_HEADER_BYTES: usize = 8192;
const READ_TIMEOUT: Duration = Duration::from_secs(2);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const WAKE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_ACCEPT_ERRORS: u32 = 8;
const THREAD_NAME: &str = "native-publication-http-fixture";

pub(super) struct HttpFixture {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
    worker: Option<JoinHandle<io::Result<()>>>,
}

pub(super) fn bind() -> io::Result<HttpFixture> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(AtomicUsize::new(0));
    let worker_stop = Arc::clone(&stop);
    let worker_requests = Arc::clone(&requests);
    let worker = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || serve(listener, worker_stop, worker_requests))?;
    Ok(HttpFixture {
        address,
        stop,
        requests,
        worker: Some(worker),
    })
}

impl HttpFixture {
    pub(super) fn origin(&self) -> Result<Url, url::ParseError> {
        Url::parse(&format!("http://{}/", self.address))
    }

    pub(super) fn stop(&mut self) -> io::Result<usize> {
        self.shutdown()
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::Acquire)
    }

    fn shutdown(&mut self) -> io::Result<usize> {
        let Some(worker) = self.worker.take() else {
            return Ok(self.request_count());
        };
        self.stop.store(true, Ordering::Release);
        let wake = TcpStream::connect_timeout(&self.address, WAKE_TIMEOUT);
        let joined = worker.join();
        let count = self.request_count();
        match joined {
            Ok(result) => result?,
            Err(_) => return Err(io::Error::other("native http fixture worker panicked")),
        }
        match wake {
            Ok(stream) => {
                drop(stream);
                Ok(count)
            }
            Err(error) if is_disconnect(&error) => Ok(count),
            Err(error) => Err(error),
        }
    }
}

impl Drop for HttpFixture {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            tracing::error!(%error, "native http fixture cleanup failed during drop");
        }
    }
}

fn serve(
    listener: TcpListener,
    stop: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
) -> io::Result<()> {
    let mut accept_errors: u32 = 0;
    let mut accepted = 0usize;
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if accepted >= MAX_REQUESTS {
            return Ok(());
        }
        match listener.accept() {
            Ok((stream, _peer)) => {
                accept_errors = 0;
                accepted = accepted
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("native fixture accept count overflow"))?;
                if stop.load(Ordering::Acquire) {
                    return Ok(());
                }
                match serve_connection(stream)? {
                    ServeOutcome::Served => {
                        requests.fetch_add(1, Ordering::AcqRel);
                    }
                    ServeOutcome::Abandoned => {}
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::Interrupted | ErrorKind::ConnectionAborted => {
                    if stop.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    accept_errors = accept_errors.saturating_add(1);
                    if accept_errors >= MAX_ACCEPT_ERRORS {
                        return Err(error);
                    }
                }
                _ => return Err(error),
            },
        }
    }
}

fn serve_connection(mut stream: TcpStream) -> io::Result<ServeOutcome> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    match read_head(&mut stream)? {
        RequestHead::Abandoned => Ok(ServeOutcome::Abandoned),
        RequestHead::Complete => match write_response(&mut stream) {
            Ok(()) => Ok(ServeOutcome::Served),
            Err(error) if is_disconnect(&error) => Ok(ServeOutcome::Abandoned),
            Err(error) => Err(error),
        },
    }
}

fn read_head(stream: &mut TcpStream) -> io::Result<RequestHead> {
    let mut buffer = [0u8; MAX_HEADER_BYTES];
    let mut filled: usize = 0;
    loop {
        let filled_bytes = match buffer.get(..filled) {
            Some(bytes) => bytes,
            None => return Ok(RequestHead::Abandoned),
        };
        if filled_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(RequestHead::Complete);
        }
        let slot = match buffer.get_mut(filled..) {
            Some(slot) => slot,
            None => return Ok(RequestHead::Abandoned),
        };
        if slot.is_empty() {
            return Ok(RequestHead::Abandoned);
        }
        match stream.read(slot) {
            Ok(0) => return Ok(RequestHead::Abandoned),
            Ok(read_bytes) => {
                filled = match filled.checked_add(read_bytes) {
                    Some(next) => next,
                    None => return Ok(RequestHead::Abandoned),
                };
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) if is_disconnect(&error) => return Ok(RequestHead::Abandoned),
            Err(error) => return Err(error),
        }
    }
}

fn write_response(stream: &mut TcpStream) -> io::Result<()> {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        BODY.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(BODY)
}

fn is_disconnect(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::BrokenPipe
            | ErrorKind::ConnectionAborted
            | ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::NotConnected
            | ErrorKind::UnexpectedEof
            | ErrorKind::WouldBlock
            | ErrorKind::TimedOut
    )
}

enum RequestHead {
    Complete,
    Abandoned,
}

enum ServeOutcome {
    Served,
    Abandoned,
}
