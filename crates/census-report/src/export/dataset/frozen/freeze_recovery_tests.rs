use super::fixtures::{school, seeded_store, TestResult};
use super::objects;
use super::stream::StreamKind;
use super::verify;
use super::{freeze_v2, to_export_dataset, CancelOutcome};
use census_store::{InputState, Table};

#[test]
fn freeze_recovery_completes_an_interrupted_freeze_without_mixing_generations() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = "freeze-recovery:interrupted";
    let directory = super::freeze::directory(&store, job)?;
    std::fs::create_dir_all(objects::directory(&directory))?;
    let staged = super::freeze::stage(&store, job, &directory)?;
    let record = store
        .get_input_record(&staged.attempt)?
        .ok_or("a staged attempt is registered as a durable input record")?;
    check!(eq; record.state(), InputState::Freezing);
    let first = to_export_dataset(&super::freeze::root_of(&staged), &store)?;
    check!(eq; first.schools.len(), 1);
    store.append(Table::Schools, &school("Second School"))?;
    let current = freeze_v2(&store, job)?;
    check!(eq; current.manifest_digest(), staged.manifest_digest.as_str());
    check!(eq; current.attempt(), staged.attempt.as_str());
    let record = store
        .get_input_record(&staged.attempt)?
        .ok_or("the recovered attempt stays registered")?;
    check!(eq; record.state(), InputState::Frozen);
    let second = to_export_dataset(&current, &store)?;
    check!(eq; second.schools.len(), 1);
    check!(eq; second.lineage.input_digest, first.lineage.input_digest);
    let error = match second.ensure_current(&store) {
        Ok(()) => return Err("historical evidence published as current".into()),
        Err(error) => error,
    };
    check!(error.to_string().contains("stale or foreign"));
    Ok(())
}

#[test]
fn freeze_recovery_retires_a_root_whose_objects_drifted() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = "freeze-recovery:drifted";
    let root = freeze_v2(&store, job)?;
    let directory = root.directory().to_path_buf();
    let objects_dir = objects::directory(&directory);
    let reference = root
        .manifest()
        .stream(StreamKind::Schools)
        .ok_or("the frozen root carries the schools stream")?;
    let page = verify::page(&objects_dir, reference)?;
    let victim = page
        .chunks()
        .first()
        .ok_or("the schools stream holds a chunk")?;
    std::fs::remove_file(objects_dir.join(&victim.sha256))?;
    let error = match freeze_v2(&store, job) {
        Ok(_) => return Err("a drifted root was reused".into()),
        Err(error) => error,
    };
    check!(error.to_string().contains(victim.sha256.as_str()));
    check!(eq; super::root::read_retained(&directory)?, None);
    let record = store
        .get_input_record(root.attempt())?
        .ok_or("a completed attempt keeps its durable record")?;
    check!(eq; record.state(), InputState::Frozen);
    let recovered = freeze_v2(&store, job)?;
    check!(ne; recovered.attempt(), root.attempt());
    let dataset = to_export_dataset(&recovered, &store)?;
    check!(eq; dataset.schools.len(), 1);
    Ok(())
}

#[test]
fn freeze_recovery_cancellation_records_abandoned_and_sweeps_objects() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let frozen_job = "freeze-recovery:frozen";
    let frozen = freeze_v2(&store, frozen_job)?;
    let frozen_directory = frozen.directory().to_path_buf();
    match super::cancel_freeze_v2(&store, frozen_job)? {
        CancelOutcome::Retained { attempt } => {
            check!(eq; attempt.as_str(), frozen.attempt());
        }
        _ => return Err("a frozen root must not be cancelled".into()),
    }
    let job = "freeze-recovery:cancelled";
    let directory = super::freeze::directory(&store, job)?;
    std::fs::create_dir_all(objects::directory(&directory))?;
    let staged = super::freeze::stage(&store, job, &directory)?;
    match super::cancel_freeze_v2(&store, job)? {
        CancelOutcome::Abandoned { attempt, swept } => {
            check!(eq; attempt.as_str(), staged.attempt.as_str());
            check!(swept >= 1, "an abandoned attempt sweeps its objects");
        }
        _ => return Err("an unfinished attempt must be abandoned".into()),
    }
    let record = store
        .get_input_record(&staged.attempt)?
        .ok_or("an abandoned attempt stays registered")?;
    check!(eq; record.state(), InputState::Abandoned);
    check!(eq; super::root::read_retained(&directory)?, None);
    check!(eq; objects::directory(&directory).read_dir()?.count(), 0);
    check!(
        super::root::read_retained(&frozen_directory)?.is_some(),
        "a frozen root keeps its pointer"
    );
    let dataset = to_export_dataset(&frozen, &store)?;
    check!(eq; dataset.schools.len(), 1);
    check!(
        matches!(
            super::cancel_freeze_v2(&store, "freeze-recovery:absent")?,
            CancelOutcome::Absent
        ),
        "an unknown job has nothing to cancel"
    );
    Ok(())
}
