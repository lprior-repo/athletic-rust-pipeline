use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::Duration;

use census_domain::model::{CanonicalSchool, ReviewCase, ReviewVerdictRecord};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::Value;

use crate::{ModelClient, ModelOptions, ReviewFamily, ReviewOptions};

pub(crate) struct Fixture {
    pub(crate) store: std::sync::Arc<Store>,
    _dir: tempfile::TempDir,
}

impl Fixture {
    pub(crate) fn school() -> (Self, ReviewCase) {
        let dir = tempfile::tempdir().expect("temporary store");
        let store = Store::open(dir.path()).expect("store opens");
        let case = add_school(&store, "Madison West");
        (
            Self {
                store: std::sync::Arc::new(store),
                _dir: dir,
            },
            case,
        )
    }
}

pub(crate) fn add_school(store: &Store, name: &str) -> ReviewCase {
    let (mut school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name);
    school.state = None;
    school.city = Some("Madison".to_string());
    school.association = Some("WIAA".to_string());
    let case = ReviewCase::pending(
        "School jurisdiction unresolved",
        school.id.as_str(),
        name,
        "no jurisdiction from any source",
    );
    store
        .append_many(Table::Schools, &[school])
        .expect("school written");
    store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .expect("case written");
    case
}

pub(crate) fn options() -> ReviewOptions {
    ReviewOptions {
        families: vec![ReviewFamily::SchoolJurisdiction],
        limit: 1024,
        dry_run: false,
    }
}

pub(crate) fn client(endpoint: &str) -> ModelClient {
    ModelClient::new(
        ModelOptions::local(endpoint, "same-model-name")
            .expect("local endpoint")
            .with_timeout(Duration::from_secs(3)),
    )
    .expect("model client")
}

pub(crate) fn batch(case: &ReviewCase, kind: &str, field: &str, value: &str) -> String {
    serde_json::json!({
        "subject_id": case.subject_id,
        "verdicts": [{ "case_id": case.id, "kind": kind, "field": field, "value": value,
            "confidence": 80, "rationale": "retained association evidence" }],
    })
    .to_string()
}

pub(crate) fn lane(contents: Vec<String>) -> (String, JoinHandle<Vec<Value>>) {
    lane_with(contents, |_, _| {})
}

pub(crate) fn lane_with(
    contents: Vec<String>,
    after_request: impl Fn(usize, &Value) + Send + 'static,
) -> (String, JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral port");
    let address = listener.local_addr().expect("bound address");
    listener.set_nonblocking(true).expect("bounded accept");
    let handle = std::thread::spawn(move || {
        contents
            .into_iter()
            .enumerate()
            .map(|(index, content)| {
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "review request never arrived"
                            );
                            std::thread::yield_now();
                        }
                        Err(error) => panic!("local accept: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .expect("read timeout");
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .expect("write timeout");
                let request = read_request(&mut stream);
                after_request(index, &request);
                write_reply(&mut stream, &content);
                request
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

pub(crate) fn write_reply(stream: &mut TcpStream, content: &str) {
    let body =
        serde_json::json!({ "choices": [{ "message": { "content": content } }] }).to_string();
    let response = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
    stream
        .write_all(response.as_bytes())
        .expect("response written");
    stream.flush().expect("response flushed");
}

pub(crate) fn read_request(stream: &mut TcpStream) -> Value {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = stream.read(&mut buffer).expect("request bytes");
        assert_ne!(count, 0, "request ended early");
        request.extend_from_slice(&buffer[..count]);
        assert!(request.len() <= 1_100_000, "bounded request");
        let Some(head) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
        };
        let body = head + 4;
        let length = String::from_utf8_lossy(&request[..head])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().expect("content length"))
            })
            .expect("known body length");
        if request.len() >= body + length {
            return serde_json::from_slice(&request[body..body + length]).expect("request JSON");
        }
    }
}

pub(crate) fn row(store: &Store) -> ReviewVerdictRecord {
    let rows = store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("verdict rows");
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().expect("consensus row")
}

pub(crate) fn audit(store: &Store) -> Value {
    serde_json::from_str(&row(store).rationale).expect("durable advice envelope")
}

pub(crate) fn state(store: &Store) -> census_domain::model::ReviewState {
    let rows = store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("case rows");
    assert_eq!(rows.len(), 1);
    rows[0].state
}
