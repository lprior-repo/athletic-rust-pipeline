use census_domain::model::{ReviewCase, ReviewState};
use census_store::Table;

use super::support::{
    batch, client, options, read_request, row, state, write_reply, Fixture, TestResult,
};
use crate::run_lanes;

fn once(
    content: String,
) -> TestResult<(
    String,
    std::thread::JoinHandle<TestResult<std::net::TcpListener>>,
)> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}", listener.local_addr()?);
    listener.set_nonblocking(true)?;
    let server = std::thread::spawn(move || -> TestResult<std::net::TcpListener> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return Err("request did not arrive".into());
                    }
                    std::thread::yield_now();
                }
                Err(error) => return Err(error.into()),
            }
        };
        stream.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(3)))?;
        let _request = read_request(&mut stream)?;
        write_reply(&mut stream, &content)?;
        Ok(listener)
    });
    Ok((endpoint, server))
}

#[test]
fn same_observed_at_reopening_applies_a_fresh_transition_and_preserves_original_receipt(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, mut case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = once(good.clone())?;
            let (second, server_b) = once(good)?;
            let clients = [client(&first)?, client(&second)?];
            let generation = fixture.store.snapshot().evidence_generation();
            check!(eq; run_lanes(&fixture.store, &clients, &options(), "same").await?.accepted, 1);
            let listeners = [
                server_a.join().map_err(|_| "first endpoint panicked")??,
                server_b.join().map_err(|_| "second endpoint panicked")??,
            ];
            let original = row(&fixture.store)?;
            let original_case = fixture.store.scan::<ReviewCase>(Table::ReviewCases)?;
            let digest = crate::compute_digest(std::slice::from_ref(&original), &original_case)?;
            let operation = format!("review:same:0:{generation}:{digest}");
            let receipt = fixture.store.receipt(&operation)?.ok_or("receipt")?;
            case.state = ReviewState::Pending;
            fixture.store.replace_many(Table::ReviewCases, &[case])?;
            let report = run_lanes(&fixture.store, &clients, &options(), "same").await?;
            check!(eq; report.accepted, 1);
            check!(eq; row(&fixture.store)?, original);
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            check!(eq; fixture.store.receipt(&operation)?, Some(receipt));
            check!(eq; fixture.store.receipt_count()?, 2);
            for listener in listeners {
                let error = match listener.accept() {
                    Err(error) => error,
                    Ok(_) => return Err("cached reopening performs no extra HTTP".into()),
                };
                check!(eq; error.kind(), std::io::ErrorKind::WouldBlock);
            }
            Ok(())
        })
}

#[test]
fn exactly_applied_checkpoint_repetition_preserves_rows_and_receipt() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = once(good.clone())?;
            let (second, server_b) = once(good)?;
            let clients = [client(&first)?, client(&second)?];
            let generation = fixture.store.snapshot().evidence_generation();
            check!(eq; run_lanes(&fixture.store, &clients, &options(), "same").await?.accepted, 1);
            let _listeners = [
                server_a.join().map_err(|_| "first endpoint panicked")??,
                server_b.join().map_err(|_| "second endpoint panicked")??,
            ];
            let verdict = row(&fixture.store)?;
            let cases = fixture.store.scan::<ReviewCase>(Table::ReviewCases)?;
            let digest = crate::compute_digest(std::slice::from_ref(&verdict), &cases)?;
            let operation = format!("review:same:0:{generation}:{digest}");
            let receipt = fixture.store.receipt(&operation)?.ok_or("receipt")?;
            let repeated = crate::review_checkpoint::commit(
                &fixture.store,
                std::slice::from_ref(&verdict),
                &cases,
                "same",
                0,
                generation,
            )?;
            check!(repeated.repeated());
            check!(eq; repeated.receipt(), &receipt);
            check!(eq; row(&fixture.store)?, verdict);
            check!(eq; fixture.store.scan::<ReviewCase>(Table::ReviewCases)?, cases);
            check!(eq; fixture.store.receipt_count()?, 1);
            Ok(())
        })
}
