use super::*;

pub(super) fn serve(
    listener: TcpListener,
    fixtures: Fixtures,
    stop: Arc<AtomicBool>,
) -> Support<BTreeMap<String, u64>> {
    let mut counters: BTreeMap<String, u64> = BTreeMap::new();
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _peer)) => handle_connection(stream, &fixtures, &mut counters, &stop)?,
            Err(error) => match error.kind() {
                io::ErrorKind::WouldBlock => thread::park_timeout(POLL_INTERVAL),
                io::ErrorKind::Interrupted | io::ErrorKind::ConnectionAborted => {}
                _ => {
                    return Err(SupportError::Io {
                        context: "accept",
                        source: error,
                    })
                }
            },
        }
    }
    Ok(counters)
}
fn handle_connection(
    mut stream: TcpStream,
    fixtures: &Fixtures,
    counters: &mut BTreeMap<String, u64>,
    stop: &AtomicBool,
) -> Support<()> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(fault("read timeout"))?;
    stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(fault("write timeout"))?;
    match read_head(&mut stream)? {
        RequestHead::Get(path) => respond_route(&mut stream, &path, fixtures, counters, stop),
        RequestHead::OtherMethod => refuse(&mut stream, 405, "Method Not Allowed", stop),
        RequestHead::TooLarge => refuse(&mut stream, 431, TOO_LARGE_REASON, stop),
        RequestHead::Malformed => refuse(&mut stream, 400, "Bad Request", stop),
        RequestHead::Idle => Ok(()),
    }
}

enum RequestHead {
    Get(String),
    OtherMethod,
    TooLarge,
    Idle,
    Malformed,
}
fn read_head(stream: &mut TcpStream) -> Support<RequestHead> {
    let mut head = Vec::with_capacity(HEAD_LIMIT);
    let mut buffer = [0u8; READ_UNIT];
    loop {
        if let Some(end) = head.windows(4).position(|window| window == b"\r\n\r\n") {
            return Ok(head
                .get(..end)
                .map_or(RequestHead::Malformed, parse_request));
        }
        if head.len() >= HEAD_LIMIT {
            return Ok(RequestHead::TooLarge);
        }
        let room = HEAD_LIMIT
            .checked_sub(head.len())
            .ok_or_else(|| SupportError::Detail("header capacity exhausted".to_owned()))?
            .min(READ_UNIT);
        let slot = match buffer.get_mut(..room) {
            Some(slot) => slot,
            None => return Ok(RequestHead::Malformed),
        };
        match stream.read(slot) {
            Ok(0) => return Ok(RequestHead::Idle),
            Ok(read_bytes) => match slot.get(..read_bytes) {
                Some(piece) => head.extend_from_slice(piece),
                None => return Ok(RequestHead::Malformed),
            },
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if client_gone(&error) => return Ok(RequestHead::Idle),
            Err(source) => {
                return Err(SupportError::Io {
                    context: "read",
                    source,
                })
            }
        }
    }
}
fn parse_request(head: &[u8]) -> RequestHead {
    let text = match std::str::from_utf8(head) {
        Ok(text) => text,
        Err(_) => return RequestHead::Malformed,
    };
    let mut fields = text.split_whitespace();
    let method = fields.next();
    let target = fields.next();
    let version = fields.next();
    match (method, target, version) {
        (Some("GET"), Some(target), Some("HTTP/1.1" | "HTTP/1.0")) => {
            match target.split('?').next() {
                Some(path) if path.starts_with('/') => RequestHead::Get(path.to_owned()),
                _ => RequestHead::Malformed,
            }
        }
        (Some(_), Some(_), Some("HTTP/1.1" | "HTTP/1.0")) => RequestHead::OtherMethod,
        _ => RequestHead::Malformed,
    }
}

fn respond_route(
    stream: &mut TcpStream,
    path: &str,
    fixtures: &Fixtures,
    counters: &mut BTreeMap<String, u64>,
    stop: &AtomicBool,
) -> Support<()> {
    match path {
        "/" => {
            count(counters, "/")?;
            respond_body(stream, HTML_TYPE, None, BOOTSTRAP, stop)
        }
        "/ok" => {
            count(counters, "/ok")?;
            respond_body(stream, TEXT_TYPE, None, OK_BODY, stop)
        }
        "/wire-over" => {
            count(counters, "/wire-over")?;
            respond_body(stream, TEXT_TYPE, Some("gzip"), &fixtures.wire_over, stop)
        }
        "/decoded-over" => {
            count(counters, "/decoded-over")?;
            respond_body(
                stream,
                TEXT_TYPE,
                Some("gzip"),
                &fixtures.decoded_over,
                stop,
            )
        }
        "/favicon.ico" => send_all(stream, NO_CONTENT_HEAD.as_bytes(), stop),
        _ => refuse(stream, 404, "Not Found", stop),
    }
}

fn count(counters: &mut BTreeMap<String, u64>, path: &'static str) -> Support<()> {
    let entry = counters.entry(path.to_owned()).or_insert(0);
    let next = entry
        .checked_add(1)
        .ok_or_else(|| SupportError::Detail(format!("counter overflow at {path}")))?;
    *entry = next;
    Ok(())
}
fn respond_body(
    stream: &mut TcpStream,
    content_type: &str,
    encoding: Option<&str>,
    body: &[u8],
    stop: &AtomicBool,
) -> Support<()> {
    let encoding = match encoding {
        Some(value) => format!("content-encoding: {value}\r\n"),
        None => String::new(),
    };
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\n{encoding}content-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    send_all(stream, head.as_bytes(), stop)?;
    send_all(stream, body, stop)
}

fn refuse(stream: &mut TcpStream, status: u16, reason: &str, stop: &AtomicBool) -> Support<()> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: text/plain\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
    );
    send_all(stream, head.as_bytes(), stop)
}
fn send_all(stream: &mut TcpStream, bytes: &[u8], stop: &AtomicBool) -> Support<()> {
    for chunk in bytes.chunks(WRITE_UNIT) {
        if stop.load(Ordering::SeqCst) {
            return Ok(());
        }
        match stream.write_all(chunk) {
            Ok(()) => {}
            Err(error) if client_gone(&error) => return Ok(()),
            Err(source) => {
                return Err(SupportError::Io {
                    context: "write",
                    source,
                })
            }
        }
    }
    Ok(())
}
fn client_gone(error: &io::Error) -> bool {
    let kind = error.kind();
    kind == io::ErrorKind::BrokenPipe
        || kind == io::ErrorKind::ConnectionReset
        || kind == io::ErrorKind::ConnectionAborted
        || kind == io::ErrorKind::WouldBlock
        || kind == io::ErrorKind::TimedOut
}
