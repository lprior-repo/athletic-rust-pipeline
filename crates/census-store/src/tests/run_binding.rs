use super::*;

fn manifest(revision: u32) -> TestResult<RunManifest> {
    let season = SchoolYear::new(2026).ok_or("2026 is a school year")?;
    let run = CensusRun::new(season, revision).ok_or("a run revision starts at one")?;
    Ok(RunManifest {
        store_identity: "c".repeat(64),
        run,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::NewYork, UsJurisdiction::Ohio],
    })
}

#[test]
fn a_binding_waits_for_the_append_lock_before_it_reads_the_retained_run() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let bound = manifest(1)?;
    let held = store.lock_appends();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::scope(|scope| -> TestResult {
        scope.spawn(|| {
            let _ = sender.send(store.bind_run(&bound));
        });
        check!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .is_err(),
            "a binding committed while another thread held the append lock, so the read of the \
             retained run and the commit that installs a binding can cross a competing binder"
        );
        drop(held);
        receiver.recv_timeout(std::time::Duration::from_secs(60))??;
        check!(
            eq;
            store.run_manifest()?.as_ref(),
            Some(&bound),
            "the binder that waited is the binder the store retains"
        );
        check!(store.integrity()?.ok, "a bound store is sound");
        Ok(())
    })
}

#[test]
fn competing_run_bindings_retain_exactly_one_durable_winner() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = manifest(1)?;
    let second = manifest(2)?;
    let barrier = std::sync::Barrier::new(2);
    let (left, right) = std::thread::scope(|scope| -> TestResult<_> {
        let left = scope.spawn(|| {
            barrier.wait();
            store.bind_run(&first)
        });
        let right = scope.spawn(|| {
            barrier.wait();
            store.bind_run(&second)
        });
        Ok((
            left.join().map_err(|_| "first binder failed")?,
            right.join().map_err(|_| "second binder failed")?,
        ))
    })?;
    let (winner, rejected, outcome) = match (left, right) {
        (Ok(()), Err(error)) => (&first, &second, error),
        (Err(error), Ok(())) => (&second, &first, error),
        other => return Err(format!("exactly one run must bind: {other:?}").into()),
    };
    match outcome {
        StoreError::Refused { detail } => {
            check!(detail.contains(&format!("2026-{}", winner.run.revision())));
            check!(detail.contains(&format!("2026-{}", rejected.run.revision())));
        }
        other => return Err(format!("wrong binding refusal: {other:?}").into()),
    }
    check!(eq; store.run_manifest()?, Some(winner.clone()));
    let generation = store.evidence_generation();
    store.bind_run(winner)?;
    check!(eq; store.evidence_generation(), generation);
    drop(store);
    let reopened = Store::open(dir.path())?;
    check!(eq; reopened.run_manifest()?, Some(winner.clone()));
    check!(eq; reopened.evidence_generation(), generation);
    Ok(())
}
