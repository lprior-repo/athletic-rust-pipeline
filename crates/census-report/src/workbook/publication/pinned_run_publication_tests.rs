use super::*;
use census_domain::model::{CensusRun, GradYear, RunManifest};
use census_domain::UsJurisdiction;
use census_store::publish::{Digest, InputFence, Intent, Request, State};
use census_store::StoreError;
use std::collections::BTreeMap;
use std::ffi::OsString;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn manifest(store: &Store, options: &Options) -> TestResult<RunManifest> {
    Ok(RunManifest {
        store_identity: store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(options.school_year, 7).ok_or("fixture run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![
            UsJurisdiction::Wisconsin,
            UsJurisdiction::Illinois,
            UsJurisdiction::Wisconsin,
        ],
    })
}

fn bundle_bytes(workbook: &Path) -> TestResult<BTreeMap<OsString, Vec<u8>>> {
    std::fs::read_dir(workbook.parent().ok_or("bundle directory")?)?
        .map(|entry| -> TestResult<_> {
            let entry = entry?;
            Ok((entry.file_name(), std::fs::read(entry.path())?))
        })
        .collect()
}

fn preserved(
    root: &Path,
    old: &Path,
    pointer: &Path,
    bytes: &BTreeMap<OsString, Vec<u8>>,
) -> TestResult {
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; current_workbook(root)?, old);
    check!(eq; bundle_bytes(old)?, *bytes);
    verify_published(old)?;
    Ok(())
}

fn request(store: &Store, root: &Path, input: InputFence, label: &str) -> TestResult<Intent> {
    let namespace = binding::namespace(root)?;
    let identity = binding::digest(&(label, &namespace))?;
    let lease_epoch = store.publication_lease(&identity)?;
    Ok(store.publication_request(Request {
        namespace,
        identity,
        expected: pointer::read(root)?,
        input,
        lease_epoch,
    })?)
}

#[derive(Clone, Copy)]
enum BindMoment {
    AfterInput,
    AfterPreparation,
}

fn late_bind(moment: BindMoment, label: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let root = store.out_dir().join("publication");
    let bytes = bundle_bytes(&old)?;
    let previous_pointer = std::fs::read_link(root.join("current"))?;
    let dataset = ExportDataset::load(&store)?;
    let coordinates = store.snapshot();
    let input = binding::input(&dataset, &store, &options)?;
    check!(eq; input.run_manifest, None);
    check!(eq; input.evidence_generation, coordinates.evidence_generation());
    check!(eq; input.derived_generation, coordinates.derived_generation());
    drop(coordinates);
    let intent = request(&store, &root, input, label)?;
    let bound = manifest(&store, &options)?;
    if matches!(moment, BindMoment::AfterInput) {
        store.bind_run(&bound)?;
    }
    let prepared = recovery::prepare(&store, &root, &intent.request.namespace)?;
    let prepared_digest = match moment {
        BindMoment::AfterInput => Some(binding::digest(&bound)?),
        BindMoment::AfterPreparation => None,
    };
    check!(eq; prepared.run_manifest, prepared_digest);
    if matches!(moment, BindMoment::AfterPreparation) {
        store.bind_run(&bound)?;
    }
    check!(matches!(
        store.publication_fence(&intent.request, &prepared.run_manifest),
        Err(StoreError::PublicationRefused { ref detail })
            if detail.contains("run manifest changed after publication input capture")
    ));
    check!(eq; store.publication_read(&intent.request.identity)?, Some(intent.clone()));
    check!(eq; intent.state, State::Requested);
    check!(eq; binding::run_manifest(&store)?, Some(binding::digest(&bound)?));
    preserved(&root, &old, &previous_pointer, &bytes)
}

#[test]
fn first_binding_after_publication_input_capture_refuses_before_pointer_switch() -> TestResult {
    late_bind(BindMoment::AfterInput, "pinned-run:after-input")
}

#[test]
fn first_binding_after_preparation_refuses_at_the_live_final_fence() -> TestResult {
    late_bind(BindMoment::AfterPreparation, "pinned-run:after-preparation")
}

#[test]
fn first_binding_after_stage_begin_preserves_every_previous_bundle_byte() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let root = store.out_dir().join("publication");
    let bytes = bundle_bytes(&old)?;
    let previous_pointer = std::fs::read_link(root.join("current"))?;
    let dataset = ExportDataset::for_job(&store, "pinned-run:stage-before-binding")?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let stage = Stage::begin(&dataset, &store, &options)?;
    store.bind_run(&manifest(&store, &options)?)?;
    let result = stage.publish(&dataset, &options, |directory| {
        crate::workbook::write_artifacts(directory, &dataset, &options, &censuses)
    });
    check!(matches!(
        result,
        Err(ReportError::Publication(
            PublicationError::InputFenceRefused { .. }
        ))
    ));
    preserved(&root, &old, &previous_pointer, &bytes)
}

#[test]
fn new_publication_input_refuses_old_unbound_archive_after_first_binding() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let dataset = ExportDataset::load(&store)?;
    store.bind_run(&manifest(&store, &options)?)?;
    let before = store.snapshot().sequence();
    check!(matches!(
        binding::input(&dataset, &store, &options),
        Err(ReportError::Publication(PublicationError::InputFenceRefused { ref detail }))
            if detail.contains("bound run differs from frozen input lineage")
    ));
    check!(eq; store.snapshot().sequence(), before);
    Ok(())
}

#[test]
fn normal_bound_input_publishes_and_reads_back_under_the_unchanged_fence() -> TestResult {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path().join("store"))?;
    let options = super::fence_tests::options(None)?;
    let bound = manifest(&store, &options)?;
    store.bind_run(&bound)?;
    let dataset = ExportDataset::for_job(&store, "pinned-run:bound-publication")?;
    let snapshot = store.snapshot();
    let input = binding::input(&dataset, &store, &options)?;
    check!(eq; input.run_manifest, Some(binding::digest(&bound)?));
    check!(eq; input.lineage, Digest::parse(&dataset.lineage.store_identity)?);
    check!(eq; input.token, binding::input_token(&dataset.lineage)?);
    check!(eq; input.recipe, binding::recipe(&dataset.lineage, &options)?);
    check!(eq; input.evidence_generation, snapshot.evidence_generation());
    check!(eq; input.derived_generation, snapshot.derived_generation());
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    let workbook = crate::workbook::build_from(&dataset, &store, &options, &censuses)?;
    let root = store.out_dir().join("publication");
    check!(eq; current_workbook(&root)?, workbook);
    verify_published(&workbook)?;
    let current = crate::export::store_lineage(&store)?;
    let opened = open_publication(&workbook, &current, options.scope, GradYear::CO2027)?;
    check!(eq; opened.dataset.lineage, dataset.lineage);
    check!(eq; opened.dataset.lineage.run, Some(bound.run));
    check!(eq; opened.dataset.lineage.cohort, Some(bound.cohort));
    check!(eq; opened.generation_digest, verify_published(&workbook)?.generation_digest);
    Ok(())
}

#[derive(Clone, Copy)]
enum PublicationBinding {
    UnboundThenBound,
    AlreadyBound,
}

fn retained_publication(binding: PublicationBinding) -> TestResult {
    let temp = tempfile::tempdir()?;
    let store_path = temp.path().join("store");
    let store = Store::open(&store_path)?;
    let options = super::fence_tests::options(None)?;
    let bound = manifest(&store, &options)?;
    if matches!(binding, PublicationBinding::AlreadyBound) {
        store.bind_run(&bound)?;
    }
    let workbook = crate::workbook::build(&store, &options)?;
    let root = store.out_dir().join("publication");
    let directory = workbook.parent().ok_or("bundle directory")?;
    let lineage = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?
        .lineage
        .clone();
    match binding {
        PublicationBinding::UnboundThenBound => {
            check!(eq; lineage.run, None);
            check!(eq; lineage.cohort, None);
        }
        PublicationBinding::AlreadyBound => {
            check!(eq; lineage.run, Some(bound.run));
            check!(eq; lineage.cohort, Some(bound.cohort));
        }
    }
    let bytes = bundle_bytes(&workbook)?;
    let previous_pointer = std::fs::read_link(root.join("current"))?;
    if matches!(binding, PublicationBinding::UnboundThenBound) {
        store.bind_run(&bound)?;
    }
    let sequence = store.snapshot().sequence();
    for _ in 0..2 {
        preserved(&root, &workbook, &previous_pointer, &bytes)?;
        check!(eq; ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?.lineage, lineage);
    }
    check!(eq; store.snapshot().sequence(), sequence);
    drop(store);
    let reopened = Store::open(&store_path)?;
    check!(eq; reopened.run_manifest()?, Some(bound));
    for _ in 0..2 {
        preserved(&root, &workbook, &previous_pointer, &bytes)?;
        check!(eq; ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?.lineage, lineage);
    }
    check!(eq; reopened.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn retained_unbound_publication_verifies_without_hydration_after_live_binding_and_reopen(
) -> TestResult {
    retained_publication(PublicationBinding::UnboundThenBound)
}

#[test]
fn retained_bound_publication_preserves_its_archive_pointer_and_bundle_on_reopen() -> TestResult {
    retained_publication(PublicationBinding::AlreadyBound)
}
