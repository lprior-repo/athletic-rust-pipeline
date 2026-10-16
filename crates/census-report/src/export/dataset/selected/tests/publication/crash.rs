use super::{publication, ExportDataset, Fixture, GradYear, Scope, TestResult};
use crate::export::DatasetLineage;
use crate::workbook::publication::crash_tests::harness;
use census_store::publish::{Digest, Intent, Root, State};
use census_store::Store;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod corruption;
mod readback;

const WORKER: &str =
    "export::dataset::selected::tests::publication::crash::selected_publication_worker";
const CHILD_STORE: &str = "CENSUS_SELECTED_PUBLICATION_STORE";
const CHILD_ARCHIVE: &str = "CENSUS_SELECTED_PUBLICATION_ARCHIVE";
const CHILD_DIGEST: &str = "CENSUS_SELECTED_PUBLICATION_DIGEST";
const CHILD_ROOT: &str = "CENSUS_SELECTED_PUBLICATION_ROOT";

type BundleBytes = BTreeMap<PathBuf, Vec<u8>>;

#[derive(Clone, Copy)]
enum Boundary {
    BeforeSwitch,
    AfterSwitch,
}

impl Boundary {
    fn checkpoint(self) -> &'static str {
        match self {
            Self::BeforeSwitch => "intent",
            Self::AfterSwitch => "renamed",
        }
    }

    fn outcome(self, bundle: Digest) -> publication::RecoveryOutcome {
        match self {
            Self::BeforeSwitch => publication::RecoveryOutcome::ExpectedRetried { bundle },
            Self::AfterSwitch => publication::RecoveryOutcome::DesiredPublished { bundle },
        }
    }
}

struct Case {
    directory: tempfile::TempDir,
    root: PathBuf,
    store: PathBuf,
    old: PathBuf,
    old_bytes: BundleBytes,
    old_lineage: DatasetLineage,
    old_witness: Value,
    lineage: DatasetLineage,
    witness: Value,
    archive_digest: String,
}

#[test]
fn selected_publication_worker() -> TestResult {
    let Some(store) = std::env::var_os(CHILD_STORE) else {
        return Ok(());
    };
    let (store, _writer) = Store::open_public_identity_qualification_v2(store)?.into_parts();
    let archive = PathBuf::from(std::env::var_os(CHILD_ARCHIVE).ok_or("worker archive")?);
    let dataset =
        ExportDataset::reopen_frozen_with_digest(&archive, &std::env::var(CHILD_DIGEST)?)?;
    let root = PathBuf::from(std::env::var_os(CHILD_ROOT).ok_or("worker publication root")?);
    let options = super::Options {
        grad_year: Some(2027),
        out: Some(root),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![
            super::UsJurisdiction::Alabama,
            super::UsJurisdiction::Wisconsin,
        ],
        school_year: super::SchoolYear::new(2026).ok_or("worker publication year")?,
    };
    let censuses = super::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    super::workbook::build_from(&dataset, &store, &options, &censuses)?;
    Err("selected publication worker returned without its requested crash".into())
}

#[tokio::test]
async fn selected_publication_abort_before_pointer_switch_recovers_complete_bundle() -> TestResult {
    exercise(Boundary::BeforeSwitch, IntegrityCase::Intact).await
}

#[tokio::test]
async fn selected_publication_abort_after_pointer_switch_recovers_complete_bundle() -> TestResult {
    exercise(Boundary::AfterSwitch, IntegrityCase::Intact).await
}

#[tokio::test]
async fn selected_publication_abort_before_pointer_switch_refuses_partial_archive_and_proofs(
) -> TestResult {
    exercise(Boundary::BeforeSwitch, IntegrityCase::Damaged).await
}

#[tokio::test]
async fn selected_publication_abort_after_pointer_switch_refuses_partial_archive_and_proofs(
) -> TestResult {
    exercise(Boundary::AfterSwitch, IntegrityCase::Damaged).await
}

#[derive(Clone, Copy)]
enum IntegrityCase {
    Intact,
    Damaged,
}

async fn prepare() -> TestResult<Case> {
    let fixture = Fixture::composed(false).await?;
    let objects = fixture.directory.path().join("objects");
    let transport = fixture.directory.path().join("transport");
    let root = fixture.directory.path().join("publication");
    std::fs::create_dir(&objects)?;
    std::fs::create_dir(&transport)?;
    let old_dataset = fixture.capture(&objects)?;
    let old_witness = super::super::witness::capture(&old_dataset)?;
    let old_lineage = old_dataset.lineage.clone();
    let old = super::publish(&fixture, &old_dataset, &root)?;
    readback::selected_bundle(&old, &old_lineage, &old_witness)?;
    let old_bytes = readback::bundle_bytes(old.parent().ok_or("old generation")?)?;
    drop(old_dataset);
    std::fs::remove_dir_all(&objects)?;
    std::fs::create_dir(&objects)?;
    fixture.append_second_meet().await?;
    let dataset = fixture.capture(&objects)?;
    let witness = super::super::witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    check!(ne; lineage, old_lineage);
    let archive_digest = dataset.archive_digest()?;
    dataset.save_frozen(&transport.join("selected.json"))?;
    let store = fixture.store.root().to_path_buf();
    let cache = fixture.store.http_cache_dir();
    drop(dataset);
    let Fixture {
        directory,
        store: owner,
        ..
    } = fixture;
    drop(owner);
    std::fs::remove_dir_all(&objects)?;
    std::fs::remove_dir_all(&cache)?;
    check!(eq; objects.exists(), false);
    check!(eq; cache.exists(), false);
    Ok(Case {
        directory,
        root,
        store,
        old,
        old_bytes,
        old_lineage,
        old_witness,
        lineage,
        witness,
        archive_digest,
    })
}

async fn exercise(boundary: Boundary, integrity: IntegrityCase) -> TestResult {
    let mut case = prepare().await?;
    let pointer = std::fs::read_link(case.root.join("current"))?;
    let mut command = worker_command(&case, boundary)?;
    let socket_directory = tempfile::tempdir_in("/tmp")?;
    let socket = socket_directory.path().join("selected.sock");
    let listener = UnixListener::bind(&socket)?;
    command.env("CENSUS_PUBLICATION_CRASH_SOCKET", &socket);
    let (status, directory) = harness::with_terminated_child(command, case.directory, |child| {
        let mut stream = harness::wait_checkpoint(
            listener,
            child,
            boundary.checkpoint(),
            harness::WaitBudget::REAL,
        )?;
        stream.write_all(&[1])?;
        harness::wait_aborted(child)
    })?;
    case.directory = directory;
    check!(eq; status.signal(), Some(6));
    std::fs::remove_dir_all(case.directory.path().join("transport"))?;
    let (store, _writer) = Store::open_public_identity_qualification_v2(&case.store)?.into_parts();
    let intent = harness::active_intent(&store, &case.root)?;
    let desired = intent
        .desired()
        .ok_or("selected desired generation")?
        .clone();
    check!(eq; intent.state, State::Intent { desired: desired.clone() });
    check!(eq; intent.request.expected, Root::Bundle(Digest::parse(
        pointer.file_name().and_then(|name| name.to_str()).ok_or("old generation digest")?,
    )?));
    let target = case
        .root
        .join("generations")
        .join(desired.as_str())
        .join("workbook.xlsx");
    check!(target != case.old);
    let visible = match boundary {
        Boundary::BeforeSwitch => &case.old,
        Boundary::AfterSwitch => &target,
    };
    check!(eq; publication::current_workbook(&case.root)?, *visible);
    readback::selected_bundle(&target, &case.lineage, &case.witness)?;
    let desired_bytes = readback::bundle_bytes(target.parent().ok_or("desired generation")?)?;
    readback::old_unchanged(&case)?;
    if matches!(integrity, IntegrityCase::Damaged) {
        corruption::refuse_each_damage(&case, &store, &intent, &target, boundary)?;
    }
    recover_and_remove_sources(
        case,
        store,
        intent,
        desired,
        target,
        desired_bytes,
        boundary,
    )
}

fn worker_command(case: &Case, boundary: Boundary) -> TestResult<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", WORKER, "--nocapture", "--test-threads=1"])
        .env(CHILD_STORE, &case.store)
        .env(
            CHILD_ARCHIVE,
            case.directory.path().join("transport/selected.json"),
        )
        .env(CHILD_DIGEST, &case.archive_digest)
        .env(CHILD_ROOT, &case.root)
        .env("CENSUS_PUBLICATION_CRASH_WINDOW", boundary.checkpoint())
        .env("CENSUS_PUBLICATION_CRASH_TERMINATION", "abort")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    Ok(command)
}

fn recover_and_remove_sources(
    case: Case,
    store: Store,
    intent: Intent,
    desired: Digest,
    target: PathBuf,
    desired_bytes: BundleBytes,
    boundary: Boundary,
) -> TestResult {
    check!(eq; publication::recover_publication(&store, &case.root)?, boundary.outcome(desired.clone()));
    check!(eq; publication::current_workbook(&case.root)?, target);
    check!(eq; store.publication_read(&intent.request.identity)?.ok_or("selected receipt")?.state,
        State::Published { receipt: desired.clone() });
    check!(eq; publication::recover_publication(&store, &case.root)?,
        publication::RecoveryOutcome::HistoricalSuccess { bundle: desired.clone() });
    check!(eq; readback::bundle_bytes(target.parent().ok_or("desired generation")?)?, desired_bytes);
    readback::old_unchanged(&case)?;
    drop(store);
    std::fs::remove_dir_all(&case.store)?;
    for absent in [
        &case.store,
        &case.directory.path().join("objects"),
        &case.directory.path().join("transport"),
    ] {
        check!(eq; absent.exists(), false);
    }
    readback::selected_bundle(&target, &case.lineage, &case.witness)?;
    readback::selected_bundle(&case.old, &case.old_lineage, &case.old_witness)?;
    check!(eq; publication::current_workbook(&case.root)?, target);
    check!(eq; readback::bundle_bytes(target.parent().ok_or("desired generation")?)?, desired_bytes);
    readback::old_unchanged(&case)?;
    eprintln!("selected publication checkpoint={} signal=6 bundle={} families=11 source_store_removed=true original_objects_removed=true byte_identical=true", boundary.checkpoint(), desired.as_str());
    Ok(())
}
