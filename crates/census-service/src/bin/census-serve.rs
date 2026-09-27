#![forbid(unsafe_code)]

use std::process::ExitCode;

use census_service::bootstrap::{serve, ServeOptions};

#[tokio::main]
async fn main() -> ExitCode {
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
                "drained: accepted={} completed={} cancelled={} timed_out={} aborted={} panicked={}",
                report.accepted,
                report.completed,
                report.cancelled,
                report.timed_out,
                report.aborted,
                report.panicked
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("census-serve: {error:#}");
            ExitCode::FAILURE
        }
    }
}
