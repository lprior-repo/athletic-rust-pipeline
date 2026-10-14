use super::*;

#[test]
fn independent_readers_agree_after_exact_names_are_unlinked() -> TestResult {
    let root = tempfile::tempdir()?;
    let expected = vec![meet(UsJurisdiction::Wisconsin, "1", Some("2024-04-01"))];
    let (dataset, _) = build(root.path(), expected.clone())?;
    check!(!root.path().join(".results-source-spill").exists());
    let left = collect(&dataset)?;
    let partition = dataset
        .partitions
        .iter()
        .find(|partition| partition.jurisdiction == UsJurisdiction::Wisconsin)
        .ok_or("WI partition")?;
    let mut first = [0_u8; 4];
    let mut second = [0_u8; 4];
    let mut shifted = [0_u8; 4];
    let count = reader::read_at(&partition.file, &mut first, 0)?;
    reader::read_at(&partition.file, &mut shifted, 1)?;
    check!(eq; reader::read_at(&partition.file, &mut second, 0)?, count);
    check!(eq; first, second);
    let right = collect(&dataset)?;
    check!(eq; left, expected);
    check!(eq; right, expected);
    Ok(())
}

#[test]
fn disk_reservation_survives_unlink_and_releases_after_last_arc_and_descriptors() -> TestResult {
    let root = tempfile::tempdir()?;
    let (dataset, admission) = build(
        root.path(),
        vec![meet(UsJurisdiction::Wisconsin, "1", None)],
    )?;
    let bytes = dataset.disk_bytes;
    let held = Arc::clone(&dataset);
    check!(eq; admission.reserved_bytes()?, bytes);
    check!(admission.claim().is_err());
    drop(dataset);
    check!(eq; admission.reserved_bytes()?, bytes);
    check!(eq; collect(&held)?.len(), 1);
    drop(held);
    check!(eq; admission.reserved_bytes()?, 0);
    drop(admission.claim()?);
    Ok(())
}

#[test]
fn foreign_name_refuses_unlink_and_poison_preserves_disk_accounting() -> TestResult {
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let mut build = Build::new(root.path(), Arc::clone(&admission))?;
    let row = meet(UsJurisdiction::Wisconsin, "1", None);
    let mut encoded = serde_json::to_vec(&wire::Borrowed::new(&row, 1)?)?;
    encoded.push(b'\n');
    build.push(row, &uncancelled())?;
    check!(eq; admission.reserved_bytes()?, 0);
    let foreign = root.path().join(".results-source-spill/foreign");
    std::fs::write(&foreign, b"foreign")?;
    let error = match build.finish(&uncancelled()) {
        Err(error) => error,
        Ok(_) => return Err("foreign namespace was accepted".into()),
    };
    check!(error.to_string().contains("namespace"));
    check!(eq; std::fs::read(&foreign)?, b"foreign".to_vec());
    check!(eq; admission.reserved_bytes()?, u64::try_from(encoded.len())?);
    check!(admission.claim().is_err());
    Ok(())
}

#[test]
fn replaced_directory_is_not_removed_and_both_failure_causes_survive() -> TestResult {
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let build = Build::new(root.path(), Arc::clone(&admission))?;
    let namespace = root.path().join(".results-source-spill");
    std::fs::rename(&namespace, root.path().join("renamed-owned"))?;
    std::fs::create_dir(&namespace)?;
    let error = build.abort(owner::refused("primary fixture failure"));
    check!(namespace.is_dir());
    check!(error.to_string().contains("primary fixture failure"));
    check!(error.to_string().contains("namespace changed identity"));
    check!(admission.claim().is_err());
    Ok(())
}

#[test]
fn crash_leftovers_are_never_adopted_or_recursively_removed() -> TestResult {
    let root = tempfile::tempdir()?;
    let namespace = root.path().join(".results-source-spill");
    std::fs::create_dir(&namespace)?;
    std::fs::write(namespace.join("wi"), b"old generation")?;
    let admission = Arc::new(SpillAdmission::default());
    check!(Build::new(root.path(), Arc::clone(&admission)).is_err());
    check!(eq; std::fs::read(namespace.join("wi"))?, b"old generation".to_vec());
    check!(admission.claim().is_err());
    Ok(())
}

#[test]
fn cancellation_cleans_provisional_files_and_releases_permits() -> TestResult {
    let root = tempfile::tempdir()?;
    let spill = Arc::new(SpillAdmission::default());
    let mut build = Build::new(root.path(), Arc::clone(&spill))?;
    let cancel = AtomicBool::new(true);
    let error = build
        .push(meet(UsJurisdiction::Wisconsin, "1", None), &cancel)
        .err()
        .ok_or("cancelled build accepted a row")?;
    let error = build.abort(error);
    check!(error.to_string().contains("cancelled"));
    check!(!root.path().join(".results-source-spill").exists());
    drop(spill.claim()?);
    let (dataset, _) = super::build(
        root.path(),
        vec![meet(UsJurisdiction::Wisconsin, "1", None)],
    )?;
    let admission = Arc::new(Admission::default());
    let stage = scope(UsJurisdiction::Wisconsin)?;
    check!(selection::partition(stage, Arc::clone(&admission), dataset, &cancel).is_err());
    check!(eq; admission.selected_usage(), (0, 0));
    Ok(())
}

#[test]
fn isolated_process_exit_closes_anonymous_spill_descriptors() -> TestResult {
    const KEY: &str = "CENSUS_RESULTS_SPILL_EXIT_FIXTURE";
    if let Some(root) = std::env::var_os(KEY) {
        let (dataset, _) = build(
            Path::new(&root),
            vec![meet(UsJurisdiction::Wisconsin, "1", None)],
        )?;
        check!(!Path::new(&root).join(".results-source-spill").exists());
        std::fs::write(
            Path::new(&root).join("child-executed"),
            dataset.disk_bytes.to_string(),
        )?;
        std::mem::forget(dataset);
        std::process::exit(0);
    }
    let root = tempfile::tempdir()?;
    let executable = std::env::current_exe()?;
    let status = std::process::Command::new(executable)
        .arg("--exact")
        .arg("restate_services::results_arms::capture::spill::tests::lifecycle_tests::isolated_process_exit_closes_anonymous_spill_descriptors")
        .env(KEY, root.path())
        .status()?;
    check!(status.success());
    let child_bytes: u64 = std::fs::read_to_string(root.path().join("child-executed"))?.parse()?;
    check!(child_bytes > 0);
    check!(!root.path().join(".results-source-spill").exists());
    let (_dataset, _) = build(root.path(), Vec::new())?;
    Ok(())
}

#[test]
fn counter_overflow_happens_before_namespace_or_disk_ownership_is_created() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(root.path())?);
    let mut cache = crate::restate_services::results_arms::capture::Cache {
        capture_attempts: u64::MAX,
        ..Default::default()
    };
    let error = cache
        .select(&store, scope(UsJurisdiction::Wisconsin)?, &uncancelled())
        .err()
        .ok_or("counter overflow accepted")?;
    check!(matches!(error, StoreError::CounterOverflow));
    check!(!root.path().join(".results-source-spill").exists());
    drop(cache.admission.spill.claim()?);
    Ok(())
}

#[test]
fn unwinding_build_guard_poison_keeps_reserved_bytes_until_cleanup_is_resolved() -> TestResult {
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let mut build = Build::new(root.path(), Arc::clone(&admission))?;
    let mut row = meet(UsJurisdiction::Wisconsin, "1", None);
    row.name = "x".repeat(budget::SPILL_BUFFER_BYTES * 2);
    build.push(row, &uncancelled())?;
    let reserved = admission.reserved_bytes()?;
    check!(reserved > 0);
    check!(eq; reserved, admission.writes().1);
    std::fs::write(
        root.path().join(".results-source-spill/foreign"),
        b"foreign",
    )?;
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _owned = build;
        std::panic::resume_unwind(Box::new("fixture unwind"));
    }));
    check!(unwind.is_err());
    check!(eq; admission.reserved_bytes()?, reserved);
    check!(admission.claim().is_err());
    Ok(())
}
