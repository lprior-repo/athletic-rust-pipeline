use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

use crate::baseline;

#[derive(Subcommand, Debug)]
pub(super) enum QualityCommand {
    #[command(about = "Rewrite the debt baseline from current measurements")]
    QualityBaseline {
        #[arg(help = "The baseline to write, e.g. `tools/quality-baseline.json`")]
        baseline: PathBuf,
        #[arg(help = "The gate's clippy tallies: `crate<TAB>lint<TAB>count` lines")]
        clippy: PathBuf,
        #[arg(help = "The `scan` report the clippy tallies are ratcheted with")]
        scan: PathBuf,
        #[arg(
            help = "Permit an increase: without it, the update refuses any number that would grow"
        )]
        #[arg(long)]
        allow_increase: bool,
    },
    #[command(
        about = "Compare current measurements against the debt baseline; fail when any metric grew"
    )]
    Ratchet {
        #[arg(help = "The baseline to compare against, e.g. `tools/quality-baseline.json`")]
        baseline: PathBuf,
        #[arg(help = "The gate's clippy tallies: `crate<TAB>lint<TAB>count` lines")]
        clippy: PathBuf,
        #[arg(help = "The `scan` report to compare with the baseline's recorded scan")]
        scan: PathBuf,
    },
}

impl QualityCommand {
    pub(super) fn run(self) -> Result<()> {
        match self {
            Self::QualityBaseline {
                baseline,
                clippy,
                scan,
                allow_increase,
            } => baseline::update(&baseline, &clippy, &scan, allow_increase),
            Self::Ratchet {
                baseline,
                clippy,
                scan,
            } => baseline::ratchet(&baseline, &clippy, &scan),
        }
    }
}
