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
    TestResult,
};
use crate::run_lanes;

type HeldLane = (
    String,
    oneshot::Receiver<()>,
    mpsc::Sender<()>,
    JoinHandle<TestResult>,
);

fn held_lane(replies: Vec<String>) -> TestResult<HeldLane> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}", listener.local_addr()?);
    listener.set_nonblocking(true)?;
    let (ready_tx, ready) = oneshot::channel();
    let (finish, finish_rx) = mpsc::channel();
    let server = std::thread::spawn(move || -> TestResult {
        for content in replies {
            let mut stream = accept(&listener)?;
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            stream.set_write_timeout(Some(Duration::from_secs(3)))?;
            read_request(&mut stream)?;
            write_reply(&mut stream, &content)?;
        }
        let mut stream = accept(&listener)?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let _request = read_request(&mut stream)?;
        ready_tx.send(()).map_err(|_| "request readiness")?;
        finish_rx.recv_timeout(Duration::from_secs(3))?;
        drop(stream);
        Ok(())
    });
    Ok((endpoint, ready, finish, server))
}

fn accept(listener: &TcpListener) -> TestResult<std::net::TcpStream> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return Ok(stream),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err("request deadline".into());
                }
                std::thread::yield_now();
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[test]
fn cancelled_dual_advice_commits_neither_half_a_vote_nor_a_case_transition() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let (first, ready_a, finish_a, server_a) = held_lane(Vec::new())?;
            let (second, ready_b, finish_b, server_b) = held_lane(Vec::new())?;
            let clients = [client(&first)?, client(&second)?];
            let options = options();
            let pending = Box::pin(run_lanes(&fixture.store, &clients, &options, "cancelled"));
            let ready = Box::pin(join(ready_a, ready_b));
            let selected =
                tokio::time::timeout(Duration::from_secs(5), select(pending, ready)).await?;
            let readiness: TestResult = match selected {
                Either::Right(((first, second), pending)) => {
                    drop(pending);
                    first.and(second).map_err(|error| error.into())
                }
                _ => Err("review completed before held lanes answered".into()),
            };
            finish_a.send(())?;
            finish_b.send(())?;
            server_a.join().map_err(|_| "first transport panicked")??;
            server_b.join().map_err(|_| "second transport panicked")??;
            readiness?;
            check!(fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?
                .is_empty());
            check!(eq; state(&fixture.store)?, ReviewState::Pending);
            check!(eq; fixture.store.receipt_count()?, 0);
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec![good])?;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options,
                "resumed",
            )
            .await?;
            server_a.join().map_err(|_| "first fresh lane panicked")??;
            server_b
                .join()
                .map_err(|_| "second fresh lane panicked")??;
            check!(eq; report.accepted, 1);
            check!(row(&fixture.store)?.accepted);
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            check!(eq; fixture.store.receipt_count()?, 1);
            Ok(())
        })
}

#[test]
fn cancellation_preserves_completed_checkpoints_without_acknowledging_the_inflight_case(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async { let (fixture, first_case) = Fixture::school()?;
let mut cases = vec![first_case];
for index in 1..=crate::CHECKPOINT_CASES {
    cases.push(add_school(&fixture.store, &format!("School {index}"))?);
}
cases.sort_by(|first, second| first.id.cmp(&second.id));
let completed = &cases[..crate::CHECKPOINT_CASES];
let replies = completed
    .iter()
    .map(|case| batch(case, "value_proposed", "state", "WI"))
    .collect::<Vec<_>>();
let (first, ready_a, finish_a, server_a) = held_lane(replies.clone())?;
let (second, ready_b, finish_b, server_b) = held_lane(replies)?;
let clients = [client(&first)?, client(&second)?];
let options = options();
let pending = Box::pin(run_lanes(&fixture.store, &clients, &options, "cancelled"));
let selected = tokio::time::timeout(
    Duration::from_secs(30),
    select(pending, Box::pin(join(ready_a, ready_b))),
)
.await?;
let readiness: TestResult = match selected {
    Either::Right(((first, second), pending)) => {
        drop(pending);
        first.and(second).map_err(|error| error.into())
    }
    _ => Err("review completed before held lanes answered".into()),
};
finish_a.send(())?;
finish_b.send(())?;
server_a.join().map_err(|_| "first transport panicked")??;
server_b.join().map_err(|_| "second transport panicked")??;
readiness?;
let rows = fixture.store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
let states = fixture.store.scan::<ReviewCase>(Table::ReviewCases)?;
check!(eq; rows.len(), crate::CHECKPOINT_CASES);
check!(eq; fixture.store.receipt_count()?, 1);
for case in completed {
    check!(rows.iter().any(|row| row.case_id == case.id && row.accepted));
    check!(eq; states.iter().find(|state| state.id == case.id).ok_or("completed case")?.state,
    ReviewState::Resolved);
}
let inflight = cases.last().ok_or("inflight case")?;
check!(!rows.iter().any(|row| row.case_id == inflight.id));
check!(eq; states.iter().find(|state| state.id == inflight.id).ok_or("inflight state")?.state,
ReviewState::Pending);
Ok(()) })
}
