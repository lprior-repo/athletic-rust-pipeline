use super::{quarantined, temporaries, TestResult};
use crate::school_address::replay::capture::{preserve, ReplayError};
use std::path::Path;
use std::sync::Barrier;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn concurrent_identical_capture_writers_reuse_the_completed_object_without_replacing_it(
) -> TestResult {
    use std::os::unix::fs::MetadataExt;

    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    let payload = vec![0x9E_u8; 32 * 1024];
    preserve(&target, &payload)?;
    let inode = std::fs::metadata(&target)?.ino();
    let writers = [payload.as_slice(); 8];

    let results = racing_writers(&target, &writers)?;

    for result in &results {
        check!(result.is_ok(), "an identical retry failed: {result:?}");
    }
    check!(eq; std::fs::metadata(&target)?.ino(), inode);
    check!(eq; std::fs::read(&target)?, payload);
    check!(
        quarantined(dir.path())?.is_empty(),
        "identical writers retained quarantine evidence"
    );
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn concurrent_distinct_capture_writers_retain_complete_displaced_evidence() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    let payloads = (0..4_u8)
        .map(|index| vec![b'a' + index; 1024 + usize::from(index) * 256])
        .collect::<Vec<_>>();
    let writers = payloads.iter().map(Vec::as_slice).collect::<Vec<_>>();

    let results = racing_writers(&target, &writers)?;

    for result in &results {
        match result {
            Ok(()) | Err(ReplayError::Refused { .. }) => {}
            Err(ReplayError::Io { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound => {}
            other => return Err(format!("unexpected concurrent writer result: {other:?}").into()),
        }
    }
    check!(
        results.iter().any(|result| result.is_ok()),
        "no concurrent writer completed"
    );
    check!(
        is_known_payload(&writers, &std::fs::read(&target)?),
        "the final capture is not one writer's complete payload"
    );
    for (path, bytes) in &quarantined(dir.path())? {
        check!(
            is_known_payload(&writers, bytes),
            "{} holds a truncated or mixed payload",
            path.display()
        );
    }
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

fn racing_writers(target: &Path, payloads: &[&[u8]]) -> TestResult<Vec<Result<(), ReplayError>>> {
    let barrier = Barrier::new(payloads.len());
    std::thread::scope(|scope| -> TestResult<Vec<Result<(), ReplayError>>> {
        let handles = payloads
            .iter()
            .map(|payload| {
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    preserve(target, payload)
                })
            })
            .collect::<Vec<_>>();
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.join().map_err(|_| "a racing writer panicked")?);
        }
        Ok(results)
    })
}

fn is_known_payload(payloads: &[&[u8]], bytes: &[u8]) -> bool {
    payloads.contains(&bytes)
}
