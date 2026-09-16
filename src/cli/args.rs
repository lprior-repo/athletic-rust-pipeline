use clap::{Args, Parser, Subcommand};
use std::{
    net::SocketAddr,
    num::{NonZeroU16, NonZeroU32},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about = "Native Restate athlete evidence pipeline")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Worker {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "127.0.0.1:19181")]
        bind: SocketAddr,
    },
    Deploy {
        #[arg(long, default_value = "http://127.0.0.1:19070/")]
        admin: String,
        #[arg(long, default_value = "http://127.0.0.1:19181/")]
        endpoint: String,
    },
    Start(Start),
    Status {
        #[arg(long, default_value = "http://127.0.0.1:18080/")]
        ingress: String,
        #[arg(long)]
        run: String,
    },
    /// Durably publish a verified snapshot; pending rows remain explicitly pending.
    Export {
        #[arg(long, default_value = "http://127.0.0.1:18080/")]
        ingress: String,
        #[arg(long)]
        run: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify source preservation and retained XLSX/JSONL result-evidence consistency.
    Verify {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        sha256: String,
    },
}

#[derive(Args)]
#[command(group(clap::ArgGroup::new("selection").required(true).args(["per_sheet", "all"])))]
pub struct Start {
    #[arg(long, default_value = "http://127.0.0.1:18080/")]
    pub ingress: String,
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub sha256: String,
    #[arg(long)]
    pub per_sheet: Option<NonZeroU32>,
    #[arg(long)]
    pub all: bool,
    #[arg(long, default_value = "64")]
    pub concurrency: NonZeroU16,
    #[arg(long, default_value = "2027-v1")]
    pub snapshot: String,
    #[arg(long, default_value = "stage")]
    pub execution: String,
}
