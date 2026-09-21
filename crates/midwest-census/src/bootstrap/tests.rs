//! Unit tests for the service shell: flag parsing, the bind refusal, and drain accounting.

use super::*;

#[test]
fn options_parse_every_flag() {
    let args = [
        "--listen",
        "127.0.0.1:9101",
        "--data-dir",
        "/tmp/db",
        "--max-concurrent",
        "3",
        "--drain-timeout",
        "7",
    ]
    .into_iter()
    .map(str::to_string);
    let options = ServeOptions::from_env(args).unwrap();
    assert_eq!(options.listen.to_string(), "127.0.0.1:9101");
    assert_eq!(options.data_dir.to_str(), Some("/tmp/db"));
    assert_eq!(options.max_concurrent, 3);
    assert_eq!(options.drain_timeout, Duration::from_secs(7));
}

#[test]
fn options_reject_unknown_flags_and_zero_concurrency() {
    let unknown = ["--nope"].into_iter().map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(unknown),
        Err(BootstrapError::UnknownFlag { .. })
    ));
    let zero = ["--max-concurrent", "0"].into_iter().map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(zero),
        Err(BootstrapError::ConcurrencyIsZero)
    ));
}

#[test]
fn options_name_the_flag_whose_value_is_missing_or_unparseable() {
    let missing = ["--listen"].into_iter().map(str::to_string);
    let error = ServeOptions::from_env(missing).expect_err("--listen needs a value");
    assert!(matches!(&error, BootstrapError::MissingValue { flag } if flag == "--listen"));
    assert!(error.to_string().contains("--listen"));

    let junk_seconds = ["--drain-timeout", "soon"].into_iter().map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(junk_seconds),
        Err(BootstrapError::DrainTimeoutNotASeconds { raw, .. }) if raw == "soon"
    ));

    let junk_address = ["--listen", "not-an-address"]
        .into_iter()
        .map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(junk_address),
        Err(BootstrapError::ListenNotAnAddress { raw, .. }) if raw == "not-an-address"
    ));

    let help = ["--help"].into_iter().map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(help),
        Err(BootstrapError::HelpRequested)
    ));
}

#[tokio::test]
async fn a_non_loopback_listen_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServeOptions {
        listen: SocketAddr::from(([0, 0, 0, 0], 0)),
        data_dir: dir.path().to_path_buf(),
        ..ServeOptions::default()
    };
    let error = supervise(options, std::future::pending::<()>())
        .await
        .expect_err("an endpoint without an identity key must not listen off-host");
    assert!(matches!(error, BootstrapError::NonLoopbackListen { .. }));
    assert!(error.to_string().contains("is not a loopback address"));
}

#[test]
fn stop_reason_round_trips_through_its_wire_byte() {
    for reason in [
        StopReason::Signal,
        StopReason::Requested,
        StopReason::ServerExit,
    ] {
        let raw = reason as u8;
        assert_eq!(raw, reason.to_raw());
        assert_eq!(StopReason::from_raw(raw), reason);
    }
    assert_eq!(StopReason::from_raw(200), StopReason::ServerExit);
}

#[tokio::test]
async fn drain_counts_aborted_tasks_after_the_deadline() {
    let mut tasks: JoinSet<()> = JoinSet::new();
    tasks.spawn(async {
        std::future::pending::<()>().await;
    });
    let report = drain(tasks, Duration::from_millis(10)).await.unwrap();
    assert_eq!(report.accepted, 1);
    assert_eq!(report.timed_out, 1);
    assert_eq!(report.aborted, 1);
    assert_eq!(report.completed, 0);
}

#[tokio::test]
async fn drain_completes_tasks_that_finish_before_the_deadline() {
    let mut tasks: JoinSet<()> = JoinSet::new();
    tasks.spawn(async {});
    tasks.spawn(async {
        tokio::time::sleep(Duration::from_millis(1)).await;
    });
    let report = drain(tasks, Duration::from_secs(5)).await.unwrap();
    assert_eq!(report.accepted, 2);
    assert_eq!(report.completed, 2);
    assert_eq!(report.aborted, 0);
}
