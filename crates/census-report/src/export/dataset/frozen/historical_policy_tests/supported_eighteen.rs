use super::*;

#[test]
fn synthetic_policy18_roots_decode_but_never_resume_under_current_policy() -> TestResult {
    for state in [
        None,
        Some(InputState::Requested),
        Some(InputState::Freezing),
        Some(InputState::Frozen),
        Some(InputState::Abandoned),
    ] {
        let seeded = seeded_store()?;
        let store = &seeded.store;
        let job = "synthetic-policy18-retained-state";
        let current = freeze_v2(store, job)?;
        let directory = current.directory();
        let mut old = current.manifest().clone();
        old.lineage.policy_revision = 18;
        old.lineage.input_generation = super::super::generation(&old.lineage)?;
        old.input_token.semantic_policy = 18;
        let encoded = serialized_bytes(&old)?;
        check!(eq; FrozenManifest::decode(&encoded)?.lineage.policy_revision, 18);
        let manifest = objects::install(&objects::directory(directory), &encoded)?;
        let inventory = verify::inventory_of(&objects::directory(directory), &manifest, &old)?;
        let attempt = serialized_digest(&("synthetic-policy18", &manifest, format!("{state:?}")))?;
        let retained = Retained { attempt, manifest };
        root::write_retained(directory, &retained)?;
        if let Some(state) = state {
            install_historical_record(store, &retained.attempt, &old, &inventory, state)?;
        }
        let before = RetainedEvidence::capture(store, directory, &[&retained.attempt])?;
        for _ in 0..2 {
            check!(
                matches!(freeze_v2(store, job), Err(ReportError::Invariant { ref detail })
                if detail.contains("historical recruiting policy is read-only"))
            );
            before.unchanged(store, directory)?;
        }
    }
    Ok(())
}
