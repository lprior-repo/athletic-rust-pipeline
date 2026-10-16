use super::super::rendezvous::{self, Phase};
use super::*;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio};

const CHILD_ROOT: &str = "R3_CAPTURE_AFTER_SEAL_TEST_ROOT";

fn interrupted_public_capture(root: &Path) -> TestResult {
    let (store, writer) = Store::open_public_identity_qualification_v2(root)?.into_parts();
    let id = operation("public-crash-after-seal")?;
    let (gate, controller) = rendezvous::pair(Phase::AfterSeal);
    std::thread::scope(|scope| -> TestResult {
        let capture_store = &store;
        let capture_writer = &writer;
        let capture_id = &id;
        let crash = scope.spawn(move || -> TestResult {
            controller.wait()?;
            let reserved = load_graph(capture_store, capture_writer.store_identity(), capture_id)?
                .ok_or("missing crash reservation")?;
            if !reserved.is_complete(capture_store)? {
                return Err("seal not complete at crash rendezvous".into());
            }
            controller.abort()?;
            Ok(())
        });
        let result = rendezvous::with_gate(gate, || writer.capture_raw_generation_v2(&store, &id));
        crash.join().map_err(|_| "crash controller panicked")??;
        result?;
        Err("capture returned instead of aborting at sealed checkpoint".into())
    })
}

#[test]
fn public_capture_seal_survives_process_abort_before_return_and_replays_exactly() -> TestResult {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        return interrupted_public_capture(Path::new(&root));
    }
    let (directory, store, writer) = enrolled()?;
    seed(&store, 513)?;
    let authority = super::concurrency::authority_state(&store, &writer)?;
    let root = directory.path().join("identity");
    drop(writer);
    drop(store);
    let (_, module) = module_path!().split_once("::").ok_or("test module path")?;
    let test = format!(
        "{module}::public_capture_seal_survives_process_abort_before_return_and_replays_exactly"
    );
    let child = Command::new(std::env::current_exe()?)
        .args(["--exact", &test, "--nocapture"])
        .env(CHILD_ROOT, &root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()?;
    let (directory, status) = super::owned_child::wait(child, directory)?;
    assert_eq!(status.signal(), Some(6));
    let root = directory.path().join("identity");
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    let id = operation("public-crash-after-seal")?;
    let reserved =
        load_graph(&store, writer.store_identity(), &id)?.ok_or("missing retained reservation")?;
    assert!(reserved.is_complete(&store)?);
    let sequence = store.snapshot().sequence();
    let charge = super::concurrency::admission_header(&store)?;
    let physical = crate::public_identity_tests::support::retained_state(&store)?;
    let replay = writer.capture_raw_generation_v2(&store, &id)?;
    assert!(matches!(
        &replay,
        RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
    ));
    assert_eq!(replay.retained().binding(), reserved.capture());
    assert_eq!(replay.retained().seal(), reserved.seal());
    assert_eq!(retained_rows(&store, replay.retained())?, 513);
    assert_eq!(store.snapshot().sequence(), sequence);
    assert_eq!(
        super::concurrency::authority_state(&store, &writer)?,
        authority
    );
    assert_eq!(super::concurrency::admission_header(&store)?, charge);
    assert_eq!(
        crate::public_identity_tests::support::retained_state(&store)?,
        physical
    );
    Ok(())
}
