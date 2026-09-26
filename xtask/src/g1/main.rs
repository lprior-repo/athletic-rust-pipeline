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

#[derive(Parser)]
#[command(name = "g1-audit")]
pub(crate) struct Args {
    #[arg(long)]
    pub(crate) raw: Vec<String>,
    #[arg(long)]
    pub(crate) evidence: Vec<String>,
    #[arg(long)]
    pub(crate) parsed: Vec<String>,
    #[arg(long, default_value_t = 3)]
    pub(crate) samples: usize,
    #[arg(long, default_value = "set")]
    pub(crate) label: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    audit::run(&args)
}
