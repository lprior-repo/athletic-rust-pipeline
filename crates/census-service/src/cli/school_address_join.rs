use super::Cli;
use anyhow::{Context, Result};
use census_service::school_address::{join_generation, parse_source_pairs, Mode, Overrides};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(
    about = "Join a verified school-directory generation into the offline store as owned postal claims; explicit --store required"
)]
pub(super) struct SchoolAddressJoinArgs {
    #[arg(
        long,
        value_name = "DIR",
        help = "Generation root holding current/ and generations/ (as produced by `school-address --out`)"
    )]
    pub(super) generation: PathBuf,
    #[arg(
        long,
        help = "Append linked claims; otherwise inspect without changing observations"
    )]
    pub(super) apply: bool,
    #[arg(
        long,
        value_name = "DIR",
        help = "Report directory; defaults to <store>/out/school-address-join"
    )]
    pub(super) out: Option<PathBuf>,
    #[arg(
        long = "evidence-url",
        value_name = "SOURCE=URL",
        help = "Capture URL override for a lane source (`nces-ccd`/`nces-pss`), repeatable; needs --evidence-date for the same source"
    )]
    pub(super) evidence_urls: Vec<String>,
    #[arg(
        long = "evidence-date",
        value_name = "SOURCE=YYYY-MM-DD",
        help = "Observation date override paired with --evidence-url"
    )]
    pub(super) evidence_dates: Vec<String>,
}

pub(super) fn run(cli: &Cli, args: &SchoolAddressJoinArgs) -> Result<()> {
    let root = cli
        .store
        .as_deref()
        .context("school-address-join requires an explicit --store and a stopped store owner")?;
    let store = Store::open(root).context("opening the school-address store")?;
    let overrides = Overrides {
        urls: parse_source_pairs(&args.evidence_urls)?,
        dates: parse_source_pairs(&args.evidence_dates)?,
    };
    let mode = if args.apply {
        Mode::Apply
    } else {
        Mode::DryRun
    };
    let report = join_generation(
        &store,
        &args.generation,
        args.out.as_deref(),
        overrides,
        mode,
    )
    .context("joining school addresses")?;

    println!("school-address-join\t{}", mode.as_str());
    println!(
        "scanned\t{}\nlinked\t{}\nalready_linked\t{}\nwebsites\t{}\nreview\t{}\nno_match\t{}\nrefused\t{}\nevidence_missing\t{}\nmissing_state\t{}",
        report.counters.scanned, report.counters.linked, report.counters.already_linked,
        report.counters.websites, report.counters.review, report.counters.no_match,
        report.counters.refused, report.counters.evidence_missing, report.counters.missing_state
    );
    println!(
        "rule_exact_name\t{}\nrule_core_name\t{}\nrule_parenthetical\t{}\nrule_parenthetical_inner\t{}\nrule_alias\t{}",
        report.counters.exact_name, report.counters.core_name, report.counters.parenthetical,
        report.counters.parenthetical_inner, report.counters.alias
    );
    println!("review_ambiguous\t{}", report.counters.ambiguous);
    println!("report\t{}", report.report);
    println!("outcomes\t{}", report.outcomes);
    Ok(())
}
