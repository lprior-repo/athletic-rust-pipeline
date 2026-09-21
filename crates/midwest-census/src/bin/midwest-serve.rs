//! `midwest-serve` — run the census as a Restate service endpoint.
//!
//! Same adapters, same store, same reports as the batch CLI; the difference is that Restate owns the
//! journal: a crash mid-ingest resumes at the last recorded step, and every write is idempotent.
//!
//! ```text
//! midwest-serve --listen 127.0.0.1:9080 --data-dir var
//! ```

#![forbid(unsafe_code)]

use std::process::ExitCode;

use midwest_census::bootstrap::{serve, ServeOptions};

#[tokio::main]
async fn main() -> ExitCode {
    let options = match ServeOptions::from_env(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("midwest-serve: {error}");
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
            eprintln!("midwest-serve: {error:#}");
            ExitCode::FAILURE
        }
    }
}
