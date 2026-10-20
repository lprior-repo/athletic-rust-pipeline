use super::*;

#[test]
fn synthetic_policy19_roots_decode_but_never_resume_under_current_policy() -> TestResult {
    supported_roots_are_read_only(19)
}

fn supported_roots_are_read_only(policy: u32) -> TestResult {
    for state in [
        None,
        Some(InputState::Requested),
        Some(InputState::Freezing),
        Some(InputState::Frozen),
        Some(InputState::Abandoned),
    ] {
        let seeded = seeded_store()?;
        let store = seeded.store;
        let job = format!("synthetic-policy{policy}-retained-state");
        let current = freeze_v2(&store, &job)?;
        let directory = current.directory().to_path_buf();
        let expected = serialized_bytes(&super::super::to_export_dataset(&current, &store)?)?;
        let mut old = current.manifest().clone();
        old.lineage.policy_revision = policy;
        old.lineage.input_generation = super::super::generation(&old.lineage)?;
        old.input_token.semantic_policy = policy;
        let encoded = serialized_bytes(&old)?;
        check!(eq; FrozenManifest::decode(&encoded)?, old);
        let manifest = objects::install(&objects::directory(&directory), &encoded)?;
        let inventory = verify::inventory_of(&objects::directory(&directory), &manifest, &old)?;
        let attempt = serialized_digest(&(
            "synthetic-read-only-policy",
            &manifest,
            format!("{state:?}"),
        ))?;
        let retained = Retained { attempt, manifest };
        root::write_retained(&directory, &retained)?;
        if let Some(state) = state {
            install_historical_record(&store, &retained.attempt, &old, &inventory, state)?;
        }
        let historical = root::FrozenRoot::new(
            directory.clone(),
            old.clone(),
            retained.manifest.clone(),
            retained.attempt.clone(),
            root::payload_bytes_of(&old)?,
        );
        let before =
            RetainedEvidence::capture(&store, &directory, &[current.attempt(), &retained.attempt])?;
        check_readback(&store, &historical, &expected, policy)?;
        before.unchanged(&store, &directory)?;
        check_attempts(&store, &job, &directory, &before)?;
        drop(store);
        let reopened = Store::open(seeded.directory.path())?;
        check_readback(&reopened, &historical, &expected, policy)?;
        before.unchanged(&reopened, &directory)?;
        check_attempts(&reopened, &job, &directory, &before)?;
        check!(eq; root::read_retained(&directory)?, Some(retained));
    }
    Ok(())
}

fn check_readback(
    store: &Store,
    root: &root::FrozenRoot,
    expected: &[u8],
    policy: u32,
) -> TestResult {
    let dataset = super::super::to_export_dataset(root, store)?;
    check!(eq; serialized_bytes(&dataset)?, expected);
    check!(eq; dataset.semantic_policy()?.revision(), policy);
    read_only(dataset.ensure_current_policy())?;
    read_only(dataset.ensure_current(store))
}

fn check_attempts(
    store: &Store,
    job: &str,
    directory: &Path,
    before: &RetainedEvidence,
) -> TestResult {
    for _ in 0..2 {
        read_only(freeze_v2(store, job))?;
        before.unchanged(store, directory)?;
        read_only(super::super::cancel_freeze_v2(store, job))?;
        before.unchanged(store, directory)?;
    }
    Ok(())
}

fn read_only<T>(result: Result<T, ReportError>) -> TestResult {
    check!(matches!(
        result,
        Err(ReportError::Invariant { ref detail })
            if detail.contains("historical recruiting policy is read-only")
    ));
    Ok(())
}
