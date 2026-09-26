//! What the lane must do with an athlete case end to end: ask a model about two retained rows, keep
//! the answer it gave, and move the case according to whether that answer decided anything.
//!
//! The lane talks to a local server, so these tests serve one canned answer over a real socket. The
//! path under test is the one an operator's pass takes — a store, a client, a retained case, a
//! durable verdict — and a mocked client would skip the request the model is actually sent.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, RetainedConflict,
    ReviewCase, ReviewState, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;

use census_store::{Store, Table};

use super::packets::pending_cases;
use super::{run_lanes, ModelClient, ModelOptions, ReviewFamily, ReviewOptions};

/// The schools and athletes a test works with: one school, two rows the merge kept apart.
struct Fixture {
    store: Store,
    _dir: tempfile::TempDir,
}

impl Fixture {
    /// A store holding one athlete pair and the conflict the merge retained for it.
    fn new() -> (Self, ReviewCase) {
        let dir = tempfile::tempdir().expect("a temporary store");
        let store = Store::open(dir.path()).expect("the store opens");
        let school = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Madison West High School",
            normalize_name("Madison West High School"),
        )
        .0
        .id;
        let boys = CanonicalAthlete::new(&school, "Jordan Smith", GradYear::CO2027, Gender::Boys);
        let girls = CanonicalAthlete::new(&school, "Jordan Smith", GradYear::CO2027, Gender::Girls);
        store
            .append_many(Table::Athletes, &[boys.clone(), girls.clone()])
            .expect("the athletes are written");
        let detail = format!(
            "same school, name and cohort as every id here: {}, {}",
            boys.id.as_str(),
            girls.id.as_str()
        );
        let case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            boys.id.as_str(),
            "Jordan Smith (Madison West High School)",
            detail.as_str(),
        );
        let conflict = RetainedConflict::new(
            ATHLETE_IDENTITY_FAMILY,
            case.subject_id.as_str(),
            case.subject.as_str(),
            case.detail.as_str(),
        );
        store
            .replace_many(Table::Conflicts, &[conflict])
            .expect("the conflict is written");
        (Self { store, _dir: dir }, case)
    }
}

/// The options a test passes: the athlete family alone, asked about once.
fn options() -> ReviewOptions {
    ReviewOptions {
        families: vec![ReviewFamily::AthleteIdentity],
        limit: 10,
        dry_run: false,
    }
}

/// A client for one stub lane.
fn client(endpoint: &str) -> ModelClient {
    ModelClient::new(ModelOptions::local(endpoint, "stub.gguf")).expect("a client for the stub")
}

/// The batch a stub lane answers with, naming the case it decided.
fn batch(case: &ReviewCase, kind: &str, field: &str, value: &str) -> String {
    serde_json::json!({
        "subject_id": case.subject_id,
        "verdicts": [{
            "case_id": case.id,
            "kind": kind,
            "field": field,
            "value": value,
            "confidence": 80,
            "rationale": "one provider id is on both rows",
        }],
    })
    .to_string()
}

/// Serve one request with one completion, and hand back the endpoint that answers for it.
fn lane(content: String) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("an ephemeral port");
    let address = listener.local_addr().expect("the bound address");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("one connection");
        read_request(&mut stream);
        let body = serde_json::json!({
            "choices": [{ "message": { "content": content } }],
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
             connection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("the answer is written");
        stream.flush().expect("the answer is flushed");
    });
    (format!("http://{address}"), handle)
}

/// Read one request off the socket, headers and body, so the answer reaches a client that has
/// finished writing.
fn read_request(stream: &mut TcpStream) {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1_024];
    loop {
        let read = stream.read(&mut buffer).expect("the request is readable");
        if read == 0 {
            return;
        }
        request.extend_from_slice(&buffer[..read]);
        if request_complete(&request) {
            return;
        }
    }
}

/// Whether a request's headers and body have both arrived.
fn request_complete(request: &[u8]) -> bool {
    let Some(head) = find(request, b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&request[..head]);
    match content_length(&headers) {
        Some(length) => request.len().saturating_sub(head) >= length,
        None => find(&request[head..], b"\r\n0\r\n\r\n").is_some(),
    }
}

/// The offset just past `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at.saturating_add(needle.len()))
}

/// The body length a request declared, when it declared one.
fn content_length(headers: &str) -> Option<usize> {
    for line in headers.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value.trim().parse::<usize>().ok();
        }
    }
    None
}

/// The one durable verdict the store holds.
fn verdict(store: &Store) -> ReviewVerdictRecord {
    let mut rows = store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("the verdicts are readable");
    assert_eq!(rows.len(), 1, "one pass, one case, one verdict");
    rows.remove(0)
}

/// The one case row the pass wrote.
fn recorded_case(store: &Store) -> ReviewCase {
    let mut rows = store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("the cases are readable");
    assert_eq!(rows.len(), 1, "one pass, one case");
    rows.remove(0)
}

#[tokio::test]
async fn same_person_is_rejected_when_gender_differs() {
    let (fixture, case) = Fixture::new();
    let (endpoint, lane) = lane(batch(&case, "value_proposed", "identity", "same_person"));
    let report = run_lanes(
        &fixture.store,
        &[client(&endpoint)],
        &options(),
        "2026-09-22",
    )
    .await
    .expect("the pass runs");
    lane.join().expect("the stub lane served its one request");

    assert_eq!(
        report.requested, 1,
        "the conflict queue is where the case is read"
    );
    assert_eq!(report.accepted, 0);
    assert_eq!(report.rejected, 1);

    let verdict = verdict(&fixture.store);
    assert_eq!(verdict.case_id, case.id);
    assert_eq!(verdict.subject_id, case.subject_id);
    assert_eq!(verdict.family, ATHLETE_IDENTITY_FAMILY);
    assert_eq!(verdict.field, "identity");
    assert_eq!(verdict.value, "same_person");
    assert!(
        !verdict.accepted,
        "the hard contradiction refuses the merge"
    );
    assert!(verdict.rationale.contains("gender_differs"));
    assert_eq!(verdict.reviewer, "stub.gguf");

    let recorded = recorded_case(&fixture.store);
    assert_eq!(recorded.id, case.id);
    assert_eq!(recorded.state, ReviewState::Retained);
}

#[tokio::test]
async fn an_insufficient_evidence_answer_is_terminal_for_its_evidence_snapshot() {
    let (fixture, case) = Fixture::new();
    let (endpoint, lane) = lane(batch(&case, "insufficient_evidence", "", ""));
    let report = run_lanes(
        &fixture.store,
        &[client(&endpoint)],
        &options(),
        "2026-09-22",
    )
    .await
    .expect("the pass runs");
    lane.join().expect("the stub lane served its one request");

    assert_eq!(report.requested, 1);
    assert_eq!(report.insufficient, 1);
    assert_eq!(report.accepted, 0);

    let verdict = verdict(&fixture.store);
    assert_eq!(verdict.kind, "insufficient_evidence");
    assert!(
        !verdict.accepted,
        "a decline records no value, because it proposes none"
    );

    let recorded = recorded_case(&fixture.store);
    assert_eq!(
        recorded.state,
        ReviewState::Retained,
        "this evidence snapshot is terminal; new evidence reopens the question with a new case id"
    );
}

#[tokio::test]
async fn a_proposal_that_is_not_an_answer_is_retained_with_what_it_said() {
    let (fixture, case) = Fixture::new();
    let (endpoint, lane) = lane(batch(&case, "value_proposed", "identity", "maybe_same"));
    let report = run_lanes(
        &fixture.store,
        &[client(&endpoint)],
        &options(),
        "2026-09-22",
    )
    .await
    .expect("the pass runs");
    lane.join().expect("the stub lane served its one request");

    assert_eq!(report.requested, 1);
    assert_eq!(report.rejected, 1);
    assert_eq!(report.accepted, 0);

    let verdict = verdict(&fixture.store);
    assert_eq!(
        verdict.value, "maybe_same",
        "the refused answer is kept as the model gave it, so an operator can read why it was refused"
    );
    assert!(!verdict.accepted);

    let recorded = recorded_case(&fixture.store);
    assert_eq!(
        recorded.state,
        ReviewState::Retained,
        "a refusal is a finding about the model, so the case stays for the operator"
    );
}

#[test]
fn a_finding_held_by_both_the_conflict_and_the_review_table_is_asked_once() {
    let (fixture, case) = Fixture::new();
    fixture
        .store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .expect("the case is written where a pass left it");

    let pending =
        pending_cases(&fixture.store, &options()).expect("the retained cases are readable");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].0.id, case.id);
    assert_eq!(pending[0].1, ReviewFamily::AthleteIdentity);
}

#[tokio::test]
async fn a_lane_killed_before_it_answers_is_counted_and_mints_no_verdict() {
    let (fixture, _case) = Fixture::new();
    let endpoint = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("an ephemeral port");
        let address = listener.local_addr().expect("the bound address");
        format!("http://{address}")
    };
    let report = run_lanes(
        &fixture.store,
        &[client(&endpoint)],
        &options(),
        "2026-09-22",
    )
    .await
    .expect("a dead lane is a finding about the run, not a failed pass");

    assert_eq!(
        report.requested, 1,
        "the case was selected and the request attempted"
    );
    assert_eq!(
        report.failed, 1,
        "a request that never arrived is counted as a failure, so the run reports what it lost"
    );
    assert_eq!(report.accepted, 0);
    assert_eq!(report.resolved(), 0, "a failure decides nothing");

    let verdicts = fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("the verdicts are readable");
    assert!(
        verdicts.is_empty(),
        "a lane failure is never a verdict: no match, no decline, nothing to adjudicate"
    );
    let cases = fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("the cases are readable");
    assert!(
        cases.is_empty(),
        "nothing closed the case, so it stays in the conflict queue and the next pass asks it again"
    );
}
