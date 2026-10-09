use anyhow::Result;
use std::path::PathBuf;

mod build;
mod matrix;
mod readback;
mod watch;

#[derive(clap::Subcommand, Debug)]
pub enum ReviewCommand {
    #[command(
        name = "review-matrix",
        about = "Render the registered source-family and jurisdiction matrix by reading the source tree: JSON on stdout, or --out <FILE>"
    )]
    Matrix {
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    #[command(
        name = "review-readback",
        about = "Independently re-read a published bundle without the writer's code: manifest artifact hashes and lengths, worksheet row and cell counts with a per-sheet content digest, and sidecar record counts; JSON on stdout, or --out <FILE>"
    )]
    Readback {
        #[arg(long, value_name = "DIR")]
        bundle: PathBuf,
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    #[command(
        name = "review-build",
        about = "Assemble the census completeness review JSON and markdown from the review directory's curated inputs and the run's retained evidence"
    )]
    Build {
        #[arg(long, value_name = "DIR")]
        dir: PathBuf,
        #[arg(long, value_name = "DIR")]
        run: PathBuf,
        #[arg(
            help = "RFC 3339 instant recorded as the review time; defaults to the run's own submit instant so the output stays a function of the inputs"
        )]
        #[arg(long, value_name = "RFC3339")]
        reviewed_at: Option<String>,
    },
    #[command(
        name = "review-watch",
        about = "Sample the durable run's open work into the review directory: once, or every --interval-secs until interrupted"
    )]
    Watch {
        #[arg(long, value_name = "DIR")]
        run: PathBuf,
        #[arg(long, value_name = "DIR")]
        dir: PathBuf,
        #[arg(long, value_name = "ORIGIN", default_value = crate::ingress::NODE_ORIGIN)]
        origin: String,
        #[arg(long, value_name = "ORIGIN", default_value = crate::ingress::ADMIN_ORIGIN)]
        admin: String,
        #[arg(long, default_value_t = 2026)]
        season: i16,
        #[arg(long, default_value_t = 1)]
        revision: u32,
        #[arg(long, default_value_t = 300)]
        interval_secs: u64,
        #[arg(long)]
        once: bool,
    },
}

impl ReviewCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Matrix { out } => matrix::run(out.as_deref()),
            Self::Readback { bundle, out } => readback::run(&bundle, out.as_deref()),
            Self::Build {
                dir,
                run,
                reviewed_at,
            } => build::run(&dir, &run, reviewed_at.as_deref()),
            Self::Watch {
                run,
                dir,
                origin,
                admin,
                season,
                revision,
                interval_secs,
                once,
            } => watch::run(&watch::WatchConfig {
                run_dir: run,
                review_dir: dir,
                origin,
                admin_origin: admin,
                season,
                revision,
                interval_secs,
                once,
            }),
        }
    }
}
