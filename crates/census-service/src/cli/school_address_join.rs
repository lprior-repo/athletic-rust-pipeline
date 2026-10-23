use super::Cli;
use anyhow::{Context, Result};
use census_domain::model::serialized_digest;
use census_domain::school_directory::nces_address::NcesArtifactReferenceV1;
use census_service::school_address::{
    join_generation_pinned, reopen_nces_address_run, Acquisition, Mode, Overrides,
};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

#[derive(Debug, Args)]
#[command(
    about = "Join the selected Store's verified NCES generation as owned postal claims; explicit --store required"
)]
pub(super) struct SchoolAddressJoinArgs {
    #[arg(
        long,
        value_name = "DIR",
        help = "Generation root holding current/ and generations/ bound by the selected NCES receipt"
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
}

fn print_join_report(report: &census_service::school_address::JoinReport) {
    println!("school-address-join\t{}", report.mode);
    println!(
        "scanned\t{}\nlinked\t{}\nalready_linked\t{}\nbackfilled\t{}\nwebsites\t{}\nreview\t{}\nno_match\t{}\nrefused\t{}\nevidence_missing\t{}\nmissing_state\t{}",
        report.counters.scanned,
        report.counters.linked,
        report.counters.already_linked,
        report.counters.backfilled,
        report.counters.websites,
        report.counters.review,
        report.counters.no_match,
        report.counters.refused,
        report.counters.evidence_missing,
        report.counters.missing_state
    );
    println!(
        "rule_exact_name\t{}\nrule_core_name\t{}\nrule_parenthetical\t{}\nrule_parenthetical_inner\t{}\nrule_alias\t{}",
        report.counters.exact_name,
        report.counters.core_name,
        report.counters.parenthetical,
        report.counters.parenthetical_inner,
        report.counters.alias
    );
    println!("review_ambiguous\t{}", report.counters.ambiguous);
    println!(
        "co_op_members\t{}\nco_op_declined\t{}",
        report.counters.co_op_members, report.counters.co_op_declined
    );
    println!("report\t{}", report.report);
    println!("outcomes\t{}", report.outcomes);
}

pub(super) fn run(cli: &Cli, args: &SchoolAddressJoinArgs) -> Result<()> {
    let root = cli
        .store
        .as_deref()
        .context("school-address-join requires an explicit --store and a stopped store owner")?;
    let (store, _writer) = Store::open_public_identity_qualification_v3(root)
        .context("opening the existing current qualification Store; stop its owner first")?
        .into_parts();
    let run = store
        .run_manifest()?
        .context("school-address-join requires a bound selected Store run")?;
    let (receipt, generation) = reopen_nces_address_run(&store, &serialized_digest(&run)?)
        .context("verifying the selected NCES receipt and retained captures")?;
    drop(generation);
    let overrides = Overrides::from_acquisitions(vec![
        acquisition(0, "nces-ccd", receipt.ccd()),
        acquisition(1, "nces-pss", receipt.pss()),
    ])?;
    let mode = if args.apply {
        Mode::Apply
    } else {
        Mode::DryRun
    };
    let report = join_generation_pinned(
        &store,
        &args.generation,
        args.out.as_deref(),
        overrides,
        receipt.address_generation_digest(),
        mode,
    )
    .context("joining school addresses")?;

    print_join_report(&report);
    Ok(())
}

fn acquisition(entry: u32, provider: &str, artifact: &NcesArtifactReferenceV1) -> Acquisition {
    Acquisition {
        entry,
        provider: provider.to_owned(),
        path: String::new(),
        url: artifact.source_url().to_owned(),
        retrieved_at: Some(artifact.captured_on().to_owned()),
    }
}
