use anyhow::{Context, Result};
use census_report::{export::ExportDataset, workbook};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

mod contract;
mod paths;

#[derive(Debug, Args)]
#[command(
    about = "Publish a checked selected profile from an existing stopped V3 qualification store; qualification-only, not reusable production activation"
)]
pub(super) struct SelectedPublicationArgs {
    #[arg(
        long,
        value_name = "JSON",
        help = "Existing installation-bound profile selection document"
    )]
    selection: PathBuf,
    #[arg(
        long,
        value_name = "NEW_DIRECTORY",
        help = "New publication directory under an existing parent; existing paths are refused"
    )]
    out: PathBuf,
}

pub(super) fn run(cli: &super::Cli, args: &SelectedPublicationArgs) -> Result<()> {
    let root = cli
        .store
        .as_deref()
        .context("selected-publication requires explicit --store and a stopped owner")?;
    let selection = super::profile::input::read_selection(&args.selection)?;
    let output = paths::output(root, &args.out)?;
    let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    let snapshot = store.snapshot();
    let profile = snapshot.selected_profile(&selection)?;
    let options = contract::options(&profile, &output)?;
    let authority = snapshot
        .accepted_public_identity_snapshot()?
        .context("selected-publication requires retained accepted authority")?;
    anyhow::ensure!(authority.qualification_version() == census_domain::public_identity_qualification::PublicIdentityQualificationVersion::RetainedAuthorityGraphV3, "selected-publication requires retained authority lifecycle 7");
    drop(authority);
    let objects = paths::Scratch::create(&output)?;
    let dataset =
        ExportDataset::from_selected_snapshot(&store, &snapshot, &selection, objects.path())?;
    contract::unsupported_input(&dataset, &profile)?;
    let censuses = workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    std::fs::create_dir(&output)
        .context("reserving new publication directory without overwrite")?;
    let path = workbook::build_from(&dataset, &store, &options, &censuses)?;
    let verified = workbook::publication::verify_published(&path)?;
    println!(
        "{}",
        serde_json::json!({
            "schema": "selected_publication_qualification_v3",
            "participation_policy_revision": 4,
            "qualification_only": true,
            "qualification_lifecycle": 7,
            "profile_id": selection.profile_id(),
            "store_identity": selection.store_identity(),
            "as_of": profile.input().as_of,
            "snapshot_sequence": snapshot.sequence(),
            "input_generation": dataset.lineage.input_generation,
            "generation_digest": verified.generation_digest,
            "workbook": path,
            "independently_verified": true
        })
    );
    Ok(())
}
