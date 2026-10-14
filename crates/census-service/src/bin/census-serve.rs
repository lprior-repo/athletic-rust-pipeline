#![forbid(unsafe_code)]

use std::process::ExitCode;

use census_service::bootstrap::{serve, DrainReport, EndpointShutdown, ServeOptions, StopReason};

fn main() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("census-serve: runtime startup failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(async {
    let options = match ServeOptions::from_env(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("census-serve: {error}");
            return ExitCode::FAILURE;
        }
    };
    match serve(options).await {
        Ok(report) => {
            println!(
                "drained: accepted={} completed={} cancelled={} timed_out={} aborted={} panicked={} failed={} still_running={} remaining={} attempts_reserved={} attempts_recorded={} attempts_unknown={} sources={} sources_settled={} attempt_ceiling={}",
                report.accepted,
                report.completed,
                report.cancelled,
                report.timed_out,
                report.aborted,
                report.panicked,
                report.failed,
                report.still_running,
                report.remaining,
                report.attempts.reserved,
                report.attempts.recorded,
                report.attempts.unknown,
                report.attempts.sources,
                report.attempts.settled,
                report.attempts.ceiling
            );
            let exit = exit_status(&report);
            if exit == ExitCode::FAILURE {
                eprintln!("census-serve: unsuccessful endpoint stop: {report:?}");
            }
            exit
        }
        Err(error) => {
            eprintln!("census-serve: {error:#}");
            ExitCode::FAILURE
        }
    }
    })
}

fn exit_status(report: &DrainReport) -> ExitCode {
    if matches!(
        report.stop_reason,
        StopReason::Signal | StopReason::Requested
    ) && report.endpoint_shutdown == EndpointShutdown::Completed
        && report.aborted == 0
        && report.panicked == 0
        && report.still_running == 0
        && report.timed_out == 0
        && report.remaining == 0
        && report.completed.checked_add(report.cancelled) == Some(report.accepted)
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                return Err(
                    format!("incomplete shutdown reported CLI success: {incomplete:?}").into(),
                );
            }
        }
        Ok(())
    }
}
