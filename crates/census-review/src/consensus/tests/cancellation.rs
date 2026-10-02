use std::net::TcpListener;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::Table;
use futures::channel::oneshot;
use futures::future::{join, select, Either};

use super::support::{
    add_school, batch, client, lane, options, read_request, row, state, write_reply, Fixture,
};
use crate::run_lanes;

type HeldLane = (
    String,
    oneshot::Receiver<()>,
    mpsc::Sender<()>,
    JoinHandle<()>,
);

fn held_lane(replies: Vec<String>) -> HeldLane {
    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral endpoint");
    let endpoint = format!(
        "http://{}",
        listener.local_addr().expect("endpoint address")
    );
    listener.set_nonblocking(true).expect("bounded accept");
    let (ready_tx, ready) = oneshot::channel();
    let (finish, finish_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        for content in replies {
            let mut stream = accept(&listener);
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("read bound");
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .expect("write bound");
            read_request(&mut stream);
            write_reply(&mut stream, &content);
        }
        let mut stream = accept(&listener);
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("read bound");
        let _request = read_request(&mut stream);
        ready_tx.send(()).expect("request readiness");
        finish_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("bounded cancellation drain");
        drop(stream);
    });
    (endpoint, ready, finish, server)
}

fn accept(listener: &TcpListener) -> std::net::TcpStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "request deadline");
                std::thread::yield_now();
            }
            Err(error) => panic!("accept: {error}"),
        }
    }
}

#[tokio::test]
async fn cancelled_dual_advice_commits_neither_half_a_vote_nor_a_case_transition() {
    let (fixture, case) = Fixture::school();
    let (first, ready_a, finish_a, server_a) = held_lane(Vec::new());
    let (second, ready_b, finish_b, server_b) = held_lane(Vec::new());
    let clients = [client(&first), client(&second)];
    let options = options();
    let pending = Box::pin(run_lanes(&fixture.store, &clients, &options, "cancelled"));
    let ready = Box::pin(join(ready_a, ready_b));
    let selected = tokio::time::timeout(Duration::from_secs(5), select(pending, ready))
        .await
        .expect("both lanes receive bounded work");
    match selected {
        Either::Right(((Ok(()), Ok(())), pending)) => drop(pending),
        _ => panic!("review completed before held lanes answered"),
    }
    finish_a.send(()).expect("release first transport");
    finish_b.send(()).expect("release second transport");
    server_a.join().expect("first transport drained");
    server_b.join().expect("second transport drained");
    assert!(fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("no half advice")
        .is_empty());
    assert_eq!(state(&fixture.store), ReviewState::Pending);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec![good]);
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options,
        "resumed",
    )
    .await
    .expect("unfinished checkpoint resumes");
    server_a.join().expect("first fresh lane");
    server_b.join().expect("second fresh lane");
    assert_eq!(report.accepted, 1);
    assert!(row(&fixture.store).accepted);
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 1);
}

#[tokio::test]
async fn cancellation_preserves_completed_checkpoints_without_acknowledging_the_inflight_case() {
    let (fixture, first_case) = Fixture::school();
    let mut cases = vec![first_case];
    cases.extend(
        (1..=crate::CHECKPOINT_CASES)
            .map(|index| add_school(&fixture.store, &format!("School {index}"))),
    );
    cases.sort_by(|first, second| first.id.cmp(&second.id));
    let completed = &cases[..crate::CHECKPOINT_CASES];
    let replies = completed
        .iter()
        .map(|case| batch(case, "value_proposed", "state", "WI"))
        .collect::<Vec<_>>();
    let (first, ready_a, finish_a, server_a) = held_lane(replies.clone());
    let (second, ready_b, finish_b, server_b) = held_lane(replies);
    let clients = [client(&first), client(&second)];
    let options = options();
    let pending = Box::pin(run_lanes(&fixture.store, &clients, &options, "cancelled"));
    let selected = tokio::time::timeout(
        Duration::from_secs(30),
        select(pending, Box::pin(join(ready_a, ready_b))),
    )
    .await
    .expect("second checkpoint receives work");
    match selected {
        Either::Right(((Ok(()), Ok(())), pending)) => drop(pending),
        _ => panic!("review completed before held lanes answered"),
    }
    finish_a.send(()).expect("release first transport");
    finish_b.send(()).expect("release second transport");
    server_a.join().expect("first transport drained");
    server_b.join().expect("second transport drained");
    let rows = fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("completed advice");
    let states = fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("case states");
    assert_eq!(rows.len(), crate::CHECKPOINT_CASES);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 1);
    for case in completed {
        assert!(rows
            .iter()
            .any(|row| row.case_id == case.id && row.accepted));
        assert_eq!(
            states
                .iter()
                .find(|state| state.id == case.id)
                .expect("completed case")
                .state,
            ReviewState::Resolved
        );
    }
    let inflight = cases.last().expect("inflight case");
    assert!(!rows.iter().any(|row| row.case_id == inflight.id));
    assert_eq!(
        states
            .iter()
            .find(|state| state.id == inflight.id)
            .expect("inflight state")
            .state,
        ReviewState::Pending
    );
}
