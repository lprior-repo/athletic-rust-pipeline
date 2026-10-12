#![forbid(unsafe_code)]

use std::process::ExitCode;

use census_service::bootstrap::{serve, ServeOptions};

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
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("census-serve: {error:#}");
            ExitCode::FAILURE
        }
    }
    })
}
