use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::Duration;

use census_domain::model::{CanonicalSchool, ReviewCase, ReviewVerdictRecord};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::Value;

use crate::{ModelClient, ModelOptions, ReviewFamily, ReviewOptions};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) struct Fixture {
    pub(crate) store: std::sync::Arc<Store>,
    _dir: tempfile::TempDir,
}

impl Fixture {
    pub(crate) fn school() -> TestResult<(Self, ReviewCase)> {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let case = add_school(&store, "Madison West")?;
        Ok((
            Self {
                store: std::sync::Arc::new(store),
                _dir: dir,
            },
            case,
        ))
    }
}

pub(crate) fn add_school(store: &Store, name: &str) -> TestResult<ReviewCase> {
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
    store.append_many(Table::Schools, &[school])?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    Ok(case)
}

pub(crate) fn options() -> ReviewOptions {
    ReviewOptions {
        families: vec![ReviewFamily::SchoolJurisdiction],
        limit: 1024,
        dry_run: false,
    }
}

pub(crate) fn client(endpoint: &str) -> TestResult<ModelClient> {
    Ok(ModelClient::new(
        ModelOptions::local(endpoint, "same-model-name")?.with_timeout(Duration::from_secs(3)),
    )?)
}

pub(crate) fn batch(case: &ReviewCase, kind: &str, field: &str, value: &str) -> String {
    serde_json::json!({
        "subject_id": case.subject_id,
        "verdicts": [{ "case_id": case.id, "kind": kind, "field": field, "value": value,
            "confidence": 80, "rationale": "retained association evidence" }],
    })
    .to_string()
}

pub(crate) fn lane(
    contents: Vec<String>,
) -> TestResult<(String, JoinHandle<TestResult<Vec<Value>>>)> {
    lane_with(contents, |_, _| Ok(()))
}

pub(crate) fn lane_with(
    contents: Vec<String>,
    after_request: impl Fn(usize, &Value) -> TestResult + Send + 'static,
) -> TestResult<(String, JoinHandle<TestResult<Vec<Value>>>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let handle = std::thread::spawn(move || {
        contents
            .into_iter()
            .enumerate()
            .map(|(index, content)| -> TestResult<Value> {
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if std::time::Instant::now() >= deadline {
                                return Err("review request never arrived".into());
                            }
                            std::thread::yield_now();
                        }
                        Err(error) => return Err(error.into()),
                    }
                };
                stream.set_read_timeout(Some(Duration::from_secs(3)))?;
                stream.set_write_timeout(Some(Duration::from_secs(3)))?;
                let request = read_request(&mut stream)?;
                after_request(index, &request)?;
                write_reply(&mut stream, &content)?;
                Ok(request)
            })
            .collect()
    });
    Ok((format!("http://{address}"), handle))
}

pub(crate) fn write_reply(stream: &mut TcpStream, content: &str) -> TestResult {
    let body =
        serde_json::json!({ "choices": [{ "message": { "content": content } }] }).to_string();
    let response = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub(crate) fn read_request(stream: &mut TcpStream) -> TestResult<Value> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err("request ended early".into());
        }
        request.extend_from_slice(&buffer[..count]);
        if request.len() > 1_100_000 {
            return Err("bounded request".into());
        }
        let Some(head) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
        };
        let body = head + 4;
        let length = String::from_utf8_lossy(&request[..head])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>())
            })
            .ok_or("known body length")??;
        let end = body.checked_add(length).ok_or("request length overflow")?;
        if request.len() >= end {
            return Ok(serde_json::from_slice(&request[body..end])?);
        }
    }
}

pub(crate) fn row(store: &Store) -> TestResult<ReviewVerdictRecord> {
    let rows = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    check!(eq; rows.len(), 1);
    Ok(rows.into_iter().next().ok_or("consensus row")?)
}

pub(crate) fn audit(store: &Store) -> TestResult<Value> {
    Ok(serde_json::from_str(&row(store)?.rationale)?)
}

pub(crate) fn state(store: &Store) -> TestResult<census_domain::model::ReviewState> {
    let rows = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; rows.len(), 1);
    Ok(rows.first().ok_or("case row")?.state)
}
