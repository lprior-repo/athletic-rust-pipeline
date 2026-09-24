//! Counted inventory of live athletic.net /Search.aspx/runSearch response bodies (G1 slice).
//!
//! Read-only. Every number printed here is measured over the verbatim bytes the pipeline
//! retained, not over a re-fetch.
//!
//! This file is the binary's argument handling; [`audit::run`] owns the run and each module below
//! holds one of the responsibilities that run has.

mod audit;
mod counts;
mod digests;
mod evidence;
mod inputs;
mod links;
mod outcomes;
mod pages;
mod parser_join;
mod perpage;
mod pyrepr;
mod report;
mod rowcensus;
mod rows;
mod samples;
mod verdicts;

use clap::Parser;

// The retained-input directories, the report label and the sample cap. Plain comments deliberately:
// clap turns doc comments into `--help` text, and the help bytes are part of the observable output.
#[derive(Parser)]
#[command(name = "g1-audit")]
pub(crate) struct Args {
    // Directories of raw `<sha256>.body` response bodies.
    #[arg(long)]
    pub(crate) raw: Vec<String>,
    // Directories of evidence records.
    #[arg(long)]
    pub(crate) evidence: Vec<String>,
    // Directories of parsed records.
    #[arg(long)]
    pub(crate) parsed: Vec<String>,
    // How many samples of each class to keep.
    #[arg(long, default_value_t = 3)]
    pub(crate) samples: usize,
    // The label the header prints.
    #[arg(long, default_value = "set")]
    pub(crate) label: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    audit::run(&args)
}
