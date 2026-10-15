use super::super::historical_fixture;
use super::{options, seed, ExportDataset, Manifest, Store, TestResult};
use crate::workbook::publication::{binding, pointer, recovery, RecoveryOutcome};
use census_store::publish::{Digest, Request, Root, State};
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn reconstructed_policy20_artifact_v3_bundle_remains_verifiable_without_rewriting_bytes(
) -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Historical V3 High")?;
    let options = options()?;
    let retained = temporary.path().join("reconstructed-policy20-v3");
    let (dataset, manifest) = historical_fixture::render_policy20(&store, &retained, &options, 3)?;
    check!(eq; manifest.schema_revision, 3);
    check!(eq; manifest.lineage.policy_revision, 20);
    check!(eq; super::super::artifact_names(3, options.grad_year)?, super::super::artifact_names(4, options.grad_year)?);
    let before = bytes(&retained)?;
    super::super::verify_directory(&retained)?;
    super::super::verify_prepared(&retained, &dataset, &options)?;
    super::super::verify_published(&retained.join("workbook.xlsx"))?;
    check!(eq; bytes(&retained)?, before);
    Ok(())
}

#[test]
fn reconstructed_policy20_v3_v4_v5_receipts_recover_before_current_publication() -> TestResult {
    for revision in [3, 4, 5] {
        for interrupted in [false, true] {
            cutover(revision, interrupted)?;
        }
    }
    Ok(())
}

fn cutover(revision: u32, interrupted: bool) -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Artifact Cutover High")?;
    let mut options = options()?;
    let dataset = ExportDataset::load(&store)?;
    let root = temporary.path().join("cutover");
    options.out = Some(root.clone());
    let (old_directory, manifest, historical) =
        install_revision(&store, &root, &options, revision)?;
    let old_bytes = bytes(&old_directory)?;
    let namespace = binding::namespace(&root)?;
    let old_input = historical_fixture::input(&store, &historical, &options, revision)?;
    check!(eq; old_input.recipe, manifest.recipe()?);
    let identity = binding::digest(&(
        &namespace,
        &historical.lineage.input_generation,
        &old_input.recipe,
    ))?;
    let old_request = store.publication_request(Request {
        namespace: namespace.clone(),
        identity: identity.clone(),
        expected: Root::Absent,
        input: old_input.clone(),
        lease_epoch: store.publication_lease(&identity)?,
    })?;
    let receipt = Digest::parse(&manifest.generation_digest)?;
    let fence = store.publication_fence(&old_request.request, &old_input.run_manifest)?;
    let intent = fence.begin(fence.intent(), &receipt)?;
    pointer::switch(&root, &receipt)?;
    crate::workbook::publication::sync_directory(&root)?;
    if !interrupted {
        fence.published(&intent, &receipt)?;
    }
    drop(fence);
    pointer::validate_binding(&root, &receipt, &old_input)?;
    let current_input = binding::input(&dataset, &store, &options)?;
    check!(pointer::validate_binding(&root, &receipt, &current_input).is_err());
    let outcome = recovery::recover_publication(&store, &root)?;
    check!(matches!(
        outcome,
        RecoveryOutcome::HistoricalSuccess { .. } | RecoveryOutcome::DesiredPublished { .. }
    ));
    let preserved = store
        .publication_read(&identity)?
        .ok_or("missing old receipt")?;
    let lease = store.get_lease_record(identity.as_str())?;
    check!(matches!(preserved.state, State::Published { .. }));
    let censuses =
        crate::workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions);
    let current = crate::workbook::build_from(&dataset, &store, &options, &censuses)?;
    check!(ne; current.parent(), Some(old_directory.as_path()));
    check!(eq; super::super::verify_directory(current.parent().ok_or("new directory")?)?.schema_revision, crate::workbook::publication::ARTIFACT_REVISION);
    check!(eq; store.publication_read(&identity)?, Some(preserved));
    check!(eq; store.get_lease_record(identity.as_str())?, lease);
    check!(eq; bytes(&old_directory)?, old_bytes);
    check!(eq; crate::workbook::build_from(&dataset, &store, &options, &censuses)?, current);
    Ok(())
}

#[test]
fn artifact_revision_substitution_cannot_reuse_the_current_publication_fence() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Artifact Revision Tamper High")?;
    let options = options()?;
    let current = crate::workbook::build(&store, &options)?;
    let dataset = frozen(&current)?;
    let input = binding::input(&dataset, &store, &options)?;
    let root = temporary.path().join("substituted-revision");
    let (directory, mut manifest, old_dataset) = install_revision(&store, &root, &options, 3)?;
    let retained = bytes(&directory)?;
    super::super::verify_directory(&directory)?;
    check!(
        pointer::validate_binding(&root, &Digest::parse(&manifest.generation_digest)?, &input)
            .is_err()
    );
    check!(eq; bytes(&directory)?, retained);
    let historical = historical_fixture::input(&store, &old_dataset, &options, 3)?;
    manifest.schema_revision = 4;
    manifest.generation_digest = manifest.digest()?;
    std::fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(&manifest)?,
    )?;
    let substituted = root.join("generations").join(&manifest.generation_digest);
    std::fs::rename(directory, &substituted)?;
    super::super::verify_directory(&substituted)?;
    check!(pointer::validate_binding(
        &root,
        &Digest::parse(&manifest.generation_digest)?,
        &historical
    )
    .is_err());
    check!(super::super::verify_published(&substituted.join("workbook.xlsx")).is_err());
    super::super::verify_published(&current)?;
    Ok(())
}

fn install_revision(
    store: &Store,
    root: &Path,
    options: &crate::workbook::Options,
    revision: u32,
) -> TestResult<(std::path::PathBuf, Manifest, ExportDataset)> {
    let staging = root.join(format!("reconstructed-policy20-artifact{revision}"));
    std::fs::create_dir_all(root.join("generations"))?;
    let (dataset, manifest) =
        historical_fixture::render_policy20(store, &staging, options, revision)?;
    let installed = root.join("generations").join(&manifest.generation_digest);
    std::fs::rename(staging, &installed)?;
    Ok((installed, manifest, dataset))
}

fn frozen(workbook: &Path) -> TestResult<ExportDataset> {
    let directory = workbook.parent().ok_or("missing rendered directory")?;
    Ok(ExportDataset::reopen_frozen(
        &directory.join("frozen-input.json"),
    )?)
}

fn bytes(directory: &Path) -> TestResult<BTreeMap<String, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            Ok((
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path())?,
            ))
        })
        .collect()
}
