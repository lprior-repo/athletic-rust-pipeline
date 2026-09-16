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
        /// Search every real source row against both track and cross-country.
        #[arg(long, conflicts_with = "include_xc")]
        all_workbook_rows: bool,
        /// Restrict exhaustive matching to the actual first worksheet.
        #[arg(long, requires = "all_workbook_rows")]
        first_worksheet_only: bool,
        /// Extract and rank deterministically without contacting either model.
        #[arg(long, requires = "all_workbook_rows")]
        no_ai: bool,
        /// Reuse discovery from a stopped, compatible deterministic run; never reuse its decisions.
        #[arg(long, requires = "all_workbook_rows", conflicts_with = "no_ai")]
        reuse_searches_from: Option<PathBuf>,
        /// Include Cross Country rows in addition to configured sports.
        #[arg(long)]
        include_xc: bool,
        #[arg(long)]
        i_have_written_authorization: bool,
    },
    /// Collect the authorized, API-backed Class-of-2027 alpha source.
    CollectAuthorized {
        #[arg(long)]
        alpha_config: PathBuf,
        #[arg(long, default_value = "out-authorized-2027")]
        out_dir: PathBuf,
        #[arg(long)]
        max_units: Option<usize>,
        #[arg(long)]
        i_have_alpha_authorization: bool,
    },
    /// Match an existing workbook against a local authorized alpha source.
    MatchAuthorized {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        alpha_source: PathBuf,
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "out-authorized-matches")]
        out_dir: PathBuf,
        #[arg(long)]
        max: Option<usize>,
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
