use census_domain::model::{ReviewCase, ReviewState};
use census_store::{StoreError, Table};

use super::support::{batch, client, options, read_request, row, state, write_reply, Fixture};
use crate::run_lanes;

fn once(content: String) -> (String, std::thread::JoinHandle<std::net::TcpListener>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
    let endpoint = format!("http://{}", listener.local_addr().expect("address"));
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "request did not arrive"
                    );
                    std::thread::yield_now();
                }
                Err(error) => panic!("accept failed: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .expect("read deadline");
        stream
            .set_write_timeout(Some(std::time::Duration::from_secs(3)))
            .expect("write deadline");
        let _request = read_request(&mut stream);
        write_reply(&mut stream, &content);
        listener
    });
    (endpoint, server)
}

#[tokio::test]
async fn same_observed_at_reopening_refuses_false_success_and_preserves_original_receipt() {
    let (fixture, mut case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = once(good.clone());
    let (second, server_b) = once(good);
    let clients = [client(&first), client(&second)];
    assert_eq!(
        run_lanes(&fixture.store, &clients, &options(), "same")
            .await
            .expect("original")
            .accepted,
        1
    );
    let listeners = [
        server_a.join().expect("first endpoint"),
        server_b.join().expect("second endpoint"),
    ];
    let original = row(&fixture.store);
    let original_case = fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("case");
    let digest =
        crate::compute_digest(std::slice::from_ref(&original), &original_case).expect("digest");
    let operation = format!("review:same:0:{digest}");
    let receipt = fixture
        .store
        .receipt(&operation)
        .expect("receipt lookup")
        .expect("receipt");
    case.state = ReviewState::Pending;
    fixture
        .store
        .replace_many(Table::ReviewCases, &[case])
        .expect("explicit reopening");
    let error = run_lanes(&fixture.store, &clients, &options(), "same")
        .await
        .expect_err("cannot acknowledge unpersisted transition");
    match error {
        StoreError::Invariant { detail } => assert_eq!(detail, "repeated review checkpoint differs from durable verdict or case state; explicit projection repair required"),
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(row(&fixture.store), original);
    assert_eq!(state(&fixture.store), ReviewState::Pending);
    assert_eq!(
        fixture.store.receipt(&operation).expect("receipt lookup"),
        Some(receipt)
    );
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 1);
    for listener in listeners {
        assert_eq!(
            listener
                .accept()
                .expect_err("cached reopening performs no extra HTTP")
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[tokio::test]
async fn exactly_applied_checkpoint_repetition_preserves_rows_and_receipt() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = once(good.clone());
    let (second, server_b) = once(good);
    let clients = [client(&first), client(&second)];
    assert_eq!(
        run_lanes(&fixture.store, &clients, &options(), "same")
            .await
            .expect("original")
            .accepted,
        1
    );
    let _listeners = [
        server_a.join().expect("first endpoint"),
        server_b.join().expect("second endpoint"),
    ];
    let verdict = row(&fixture.store);
    let cases = fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("case");
    let digest = crate::compute_digest(std::slice::from_ref(&verdict), &cases).expect("digest");
    let operation = format!("review:same:0:{digest}");
    let receipt = fixture
        .store
        .receipt(&operation)
        .expect("receipt lookup")
        .expect("receipt");
    let repeated = crate::review_checkpoint::commit(
        &fixture.store,
        std::slice::from_ref(&verdict),
        &cases,
        "same",
        0,
        fixture.store.snapshot().sequence(),
    )
    .expect("already applied checkpoint stays idempotent");
    assert!(repeated.repeated());
    assert_eq!(repeated.receipt(), &receipt);
    assert_eq!(row(&fixture.store), verdict);
    assert_eq!(
        fixture
            .store
            .scan::<ReviewCase>(Table::ReviewCases)
            .expect("case"),
        cases
    );
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 1);
}
