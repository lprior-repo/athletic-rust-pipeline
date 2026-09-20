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
    /// Start or resume the durable headed-browser readiness workflow.
    BrowserStart {
        #[arg(long, default_value = "http://127.0.0.1:18080/")]
        ingress: String,
    },
    /// Observe browser challenge/human/cooldown state through Restate.
    BrowserStatus {
        #[arg(long, default_value = "http://127.0.0.1:18080/")]
        ingress: String,
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
        /// Existing stopped-writer ArtifactStore directory containing retained raw evidence.
        #[arg(long)]
        store: PathBuf,
    },
    /// Observe private rankings collection progress for a run.
    RankingsStatus {
        #[arg(long, default_value = "http://127.0.0.1:18080/")]
        ingress: String,
        #[arg(long)]
        run: String,
    },
    /// Pause the private rankings collection for a run.
    RankingsPause {
        #[arg(long, default_value = "http://127.0.0.1:18080/")]
        ingress: String,
        #[arg(long)]
        run: String,
    },
    /// Resume the private rankings collection for a run (with browser recovery).
    RankingsResume {
        #[arg(long, default_value = "http://127.0.0.1:18080/")]
        ingress: String,
        #[arg(long)]
        run: String,
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
    /// Enable rankings acquisition for the 2026 USA Grade 11 division scope
    #[arg(long)]
    pub rankings: bool,
    /// Source division gender for rankings acquisition
    #[arg(long, default_value = "m", value_parser = ["m", "f"], requires = "rankings")]
    pub rankings_gender: String,
    /// Source season kind for rankings acquisition
    #[arg(
        long,
        default_value = "outdoor",
        value_parser = ["outdoor", "indoor"],
        requires = "rankings"
    )]
    pub rankings_season: String,
    /// Max pages per event (bounded safety cap 10000)
    #[arg(long, default_value = "10000")]
    pub max_pages_per_event: u32,
    /// Publish a verified final workbook automatically after run completion.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_start(extra: &[&str]) -> Result<Cli, clap::Error> {
        let mut args = vec![
            "athletic-rust-pipeline",
            "start",
            "--input",
            "input.xlsx",
            "--sha256",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--all",
        ];
        args.extend_from_slice(extra);
        Cli::try_parse_from(args)
    }

    #[test]
    fn division_flags_without_rankings_are_rejected() {
        let cases: [&[&str]; 3] = [
            &["--rankings-gender", "f"],
            &["--rankings-season", "indoor"],
            &["--rankings-gender", "f", "--rankings-season", "indoor"],
        ];
        for flags in cases {
            let Err(error) = parse_start(flags) else {
                panic!("division flags must require --rankings: {flags:?}");
            };
            assert!(
                error.to_string().contains("--rankings"),
                "the rejection must name the missing flag, got: {error}"
            );
        }
    }

    #[test]
    fn explicit_division_selection_survives_parsing() {
        let cli = parse_start(&[
            "--rankings",
            "--rankings-gender",
            "f",
            "--rankings-season",
            "indoor",
        ])
        .expect("division selection with --rankings is valid");
        let Command::Start(start) = cli.command else {
            panic!("expected the start command");
        };
        assert_eq!(start.rankings_gender, "f");
        assert_eq!(start.rankings_season, "indoor");
    }

    #[test]
    fn rankings_alone_keeps_the_outdoor_boys_defaults() {
        let cli = parse_start(&["--rankings"]).expect("--rankings alone is valid");
        let Command::Start(start) = cli.command else {
            panic!("expected the start command");
        };
        assert_eq!(start.rankings_gender, "m");
        assert_eq!(start.rankings_season, "outdoor");
    }
}
