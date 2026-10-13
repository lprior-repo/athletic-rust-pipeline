use crate::{Store, StoreSnapshot};
use census_domain::model::RunManifest;
use std::sync::mpsc::{sync_channel, RecvTimeoutError};
use std::time::Duration;

use super::process::bounded_case;
use super::support::{equal, fixture_dir, manifest, seed, TestResult, Witness};

fn verify_view(
    view: &StoreSnapshot<'_>,
    expected: Option<RunManifest>,
    before: &Witness,
) -> TestResult {
    equal(view.run_manifest()?, expected)?;
    equal(view.sequence(), before.sequence)?;
    equal(
        (view.evidence_generation(), view.derived_generation()),
        (before.evidence, before.derived),
    )
}

#[derive(Clone, Copy)]
enum BindingState {
    Unbound,
    Bound,
}

fn run_fenced(initial: BindingState) -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let candidate = manifest(store.store_identity()?.as_str(), 1)?;
    let expected = match initial {
        BindingState::Unbound => None,
        BindingState::Bound => {
            store.bind_run(&candidate)?;
            Some(candidate.clone())
        }
    };
    let before = Witness::capture(&store)?;
    let fence = store.fenced_snapshot();
    verify_view(fence.view(), expected.clone(), &before)?;
    let (begun_tx, begun_rx) = sync_channel(1);
    let (done_tx, done_rx) = sync_channel(1);
    std::thread::scope(|scope| -> TestResult {
        let binder = scope.spawn(|| -> TestResult {
            begun_tx.send(())?;
            done_tx.send(store.bind_run(&candidate))?;
            Ok(())
        });
        let observed = (|| -> TestResult {
            begun_rx.recv_timeout(Duration::from_secs(5))?;
            match done_rx.recv_timeout(Duration::from_millis(100)) {
                Err(RecvTimeoutError::Timeout) => {}
                observed => {
                    return Err(format!(
                        "binder completed or disconnected while fence held: {observed:?}"
                    )
                    .into())
                }
            }
            verify_view(fence.view(), expected, &before)
        })();
        drop(fence);
        let drained = done_rx.recv_timeout(Duration::from_secs(5));
        let joined = binder.join().map_err(|_| "fenced binder panicked")?;
        joined?;
        observed?;
        drained??;
        Ok(())
    })?;
    equal(store.snapshot().run_manifest()?, Some(candidate))?;
    let after = Witness::capture(&store)?;
    match initial {
        BindingState::Bound => equal(after, before),
        BindingState::Unbound => before.binding_only(
            &after,
            &serde_json::to_vec(&store.run_manifest()?.ok_or("binding disappeared")?)?,
        ),
    }
}

#[test]
fn fenced_run_reads_complete_without_reentering_append_serialization() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::fenced_run_reads_complete_without_reentering_append_serialization"
        ),
        || {
            run_fenced(BindingState::Unbound)?;
            run_fenced(BindingState::Bound)
        },
    )
}
