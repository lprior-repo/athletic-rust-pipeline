use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Stream and report every real workbook row; performs no network access.
    Inspect {
        #[arg(long)]
        input: PathBuf,
    },
    /// Export every real source row to JSONL; performs no network access.
    ExportRecords {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Discover candidates, optionally retrieve pages, validate locally, and checkpoint.
    Run {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "out")]
        out_dir: PathBuf,
        #[arg(long)]
        max: Option<usize>,
        /// Include Cross Country rows in addition to configured sports.
        #[arg(long)]
        include_xc: bool,
        #[arg(long)]
        i_have_written_authorization: bool,
    },
    /// Add an Athletic Matches worksheet to a copy of the source workbook.
    Writeback {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        matches: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}
