use std::process::ExitCode;

use anyhow::Result;

use super::{serve, DrainReport, EndpointShutdown, ServeOptions, StopReason};

pub async fn serve_command(options: ServeOptions) -> Result<()> {
    let report = serve(options).await?;
    println!("{}", drain_receipt(&report));
    if exit_status(&report) == ExitCode::FAILURE {
        anyhow::bail!("unsuccessful endpoint stop: {report:?}");
    }
    Ok(())
}

fn drain_receipt(report: &DrainReport) -> String {
    format!(
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
    )
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
mod tests;
