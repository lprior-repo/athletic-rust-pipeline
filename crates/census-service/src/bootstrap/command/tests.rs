use super::*;

#[test]
fn drain_receipt_keeps_the_existing_field_order_and_values(
) -> Result<(), Box<dyn std::error::Error>> {
    let report = DrainReport {
        accepted: 15,
        completed: 2,
        cancelled: 3,
        timed_out: 4,
        aborted: 5,
        panicked: 6,
        failed: 7,
        still_running: 8,
        remaining: 9,
        attempts: crate::outcome::AttemptState {
            reserved: 10,
            recorded: 11,
            unknown: 12,
            sources: 13,
            settled: 14,
            ceiling: 3,
        },
        ..DrainReport::default()
    };
    let expected = "drained: accepted=15 completed=2 cancelled=3 timed_out=4 aborted=5 panicked=6 failed=7 still_running=8 remaining=9 attempts_reserved=10 attempts_recorded=11 attempts_unknown=12 sources=13 sources_settled=14 attempt_ceiling=3";
    if drain_receipt(&report).as_bytes() != expected.as_bytes() {
        return Err("drain receipt bytes changed".into());
    }
    Ok(())
}

#[test]
fn a_balanced_signal_shutdown_keeps_cli_success() -> Result<(), Box<dyn std::error::Error>> {
    let report = DrainReport {
        accepted: 1,
        completed: 1,
        stop_reason: StopReason::Signal,
        ..DrainReport::default()
    };
    if exit_status(&report) != ExitCode::SUCCESS {
        return Err("balanced signal shutdown did not succeed".into());
    }
    Ok(())
}

#[test]
fn resource_and_unexpected_server_stops_cannot_report_cli_success(
) -> Result<(), Box<dyn std::error::Error>> {
    for stop_reason in [StopReason::MemoryBudget, StopReason::ServerExit] {
        let report = DrainReport {
            accepted: 1,
            completed: 1,
            stop_reason,
            ..DrainReport::default()
        };
        if exit_status(&report) != ExitCode::FAILURE {
            return Err(format!("unexpected CLI success for {stop_reason:?}").into());
        }
    }
    Ok(())
}

#[test]
fn only_complete_balanced_requested_shutdowns_return_cli_success(
) -> Result<(), Box<dyn std::error::Error>> {
    let complete = DrainReport {
        accepted: 2,
        completed: 1,
        cancelled: 1,
        failed: 1,
        stop_reason: StopReason::Requested,
        ..DrainReport::default()
    };
    if exit_status(&complete) != ExitCode::SUCCESS {
        return Err(
            "balanced completed/cancelled work with a diagnostic failure was refused".into(),
        );
    }
    for incomplete in [
        DrainReport {
            aborted: 1,
            ..complete
        },
        DrainReport {
            panicked: 1,
            ..complete
        },
        DrainReport {
            still_running: 1,
            ..complete
        },
        DrainReport {
            timed_out: 1,
            ..complete
        },
        DrainReport {
            remaining: 1,
            ..complete
        },
        DrainReport {
            accepted: 3,
            ..complete
        },
        DrainReport {
            completed: u64::MAX,
            ..complete
        },
        DrainReport {
            endpoint_shutdown: EndpointShutdown::TimedOut,
            ..complete
        },
    ] {
        if exit_status(&incomplete) != ExitCode::FAILURE {
            return Err(format!("incomplete shutdown reported CLI success: {incomplete:?}").into());
        }
    }
    Ok(())
}
