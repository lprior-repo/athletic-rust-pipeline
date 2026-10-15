#![forbid(unsafe_code)]

#[path = "qualify_publication_memory/config.rs"]
mod config;
#[path = "qualify_publication_memory/fixture.rs"]
mod fixture;
#[path = "qualify_publication_memory/fixture_rows.rs"]
mod fixture_rows;
#[path = "qualify_publication_memory/identity.rs"]
mod identity;
#[path = "qualify_publication_memory/measure.rs"]
mod measure;
#[path = "qualify_publication_memory/oracle.rs"]
mod oracle;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Synthetic offline publication shape/RSS qualification; not census acceptance")]
struct Cli {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Generate {
        #[arg(long)]
        root: PathBuf,
        #[command(flatten)]
        config: config::Config,
    },
    Measure {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 3600)]
        timeout_seconds: u64,
    },
    #[command(hide = true)]
    Publish {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Operation::Generate { root, config } => fixture::generate(&root, config),
        Operation::Measure {
            root,
            out,
            timeout_seconds,
        } => measure::run(&root, &out, timeout_seconds),
        Operation::Publish { root, out } => oracle::publish(&root, &out),
    }
}

#[cfg(test)]
#[path = "qualify_publication_memory/tests.rs"]
mod tests;
