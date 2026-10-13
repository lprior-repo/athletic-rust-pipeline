use crate::{Store, StoreError};
use census_domain::model::GradYear;
use census_domain::UsJurisdiction;
use std::sync::Barrier;

use super::process::bounded_case;
use super::support::{equal, fixture_dir, manifest, raw_binding, seed, TestResult, Witness};

#[test]
fn exact_rebinding_is_read_only_and_every_manifest_difference_refuses() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let bound = manifest(store.store_identity()?.as_str(), 1)?;
    store.bind_run(&bound)?;
    let before = Witness::capture(&store)?;
    for _ in 0..3 {
        store.bind_run(&bound)?;
        equal(&Witness::capture(&store)?, &before)?;
    }
    let mut other_identity = bound.clone();
    other_identity.store_identity = "changed-identity".to_owned();
    let mut other_cohort = bound.clone();
    other_cohort.cohort = GradYear::new(2028).ok_or("invalid fixture cohort")?;
    let mut other_order = bound.clone();
    other_order.jurisdictions = vec![
        UsJurisdiction::NewYork,
        UsJurisdiction::Ohio,
        UsJurisdiction::Ohio,
    ];
    let mut deduplicated = bound.clone();
    deduplicated.jurisdictions = vec![UsJurisdiction::Ohio, UsJurisdiction::NewYork];
    for candidate in [
        manifest(&bound.store_identity, 2)?,
        other_identity,
        other_cohort,
        other_order,
        deduplicated,
    ] {
        match store.bind_run(&candidate) {
            Err(StoreError::Refused { detail }) if !detail.is_empty() => {}
            observed => {
                return Err(format!("unequal manifest must be refused: {observed:?}").into())
            }
        }
        equal(raw_binding(&store)?, Some(serde_json::to_vec(&bound)?))?;
        equal(&Witness::capture(&store)?, &before)?;
    }
    Ok(())
}

#[test]
fn competing_first_bindings_have_one_winner_and_leave_old_views_unbound() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::competing_first_bindings_have_one_winner_and_leave_old_views_unbound"
        ),
        || {
            let root = fixture_dir()?;
            let store = Store::open_with(root.path(), 8_388_608)?;
            seed(&store)?;
            let old = store.snapshot();
            let before = Witness::capture(&store)?;
            let identity = old.store_identity()?;
            let first = manifest(identity.as_str(), 1)?;
            let second = manifest(identity.as_str(), 2)?;
            let start = Barrier::new(2);
            let (left, right) = std::thread::scope(|scope| -> TestResult<_> {
                let left = scope.spawn(|| {
                    start.wait();
                    store.bind_run(&first)
                });
                let right = scope.spawn(|| {
                    start.wait();
                    store.bind_run(&second)
                });
                let left = left.join();
                let right = right.join();
                Ok((
                    left.map_err(|_| "first binder panicked")?,
                    right.map_err(|_| "second binder panicked")?,
                ))
            })?;
            let winner = match (left, right) {
                (Ok(()), Err(StoreError::Refused { .. })) => first,
                (Err(StoreError::Refused { .. }), Ok(())) => second,
                observed => {
                    return Err(format!(
                        "exactly one different first binding must succeed: {observed:?}"
                    )
                    .into())
                }
            };
            equal(old.run_manifest()?, None)?;
            equal(store.snapshot().run_manifest()?, Some(winner.clone()))?;
            let after = Witness::capture(&store)?;
            before.binding_only(&after, &serde_json::to_vec(&winner)?)?;
            store.bind_run(&winner)?;
            equal(&Witness::capture(&store)?, &after)?;
            drop(old);
            drop(store);
            let reopened = Store::open_with(root.path(), 8_388_608)?;
            equal(reopened.run_manifest()?, Some(winner.clone()))?;
            equal(reopened.snapshot().run_manifest()?, Some(winner))?;
            equal(Witness::capture(&reopened)?, after)
        },
    )
}
