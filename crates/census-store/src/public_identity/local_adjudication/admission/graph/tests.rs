mod bounds;
mod metadata;
mod recovery;
mod refusal;
mod support;

use super::super as admission;
use super::*;
use crate::public_identity::paged_generation::{
    AuthorityTableV2, CapturePositionV2, FormatBindingV2, RawPageBlockV2, RawPagePositionV2,
    RawPageV2, RawPhysicalRowV2, RawTableManifestV2, RawTableReferenceV2, RunBindingV2,
};
use crate::public_identity_tests::support::{fixture, retained_state, TestResult};
use fjall::{PersistMode, Readable};
use support::*;

#[test]
fn graph_reservation_precedes_every_child_write_and_charges_the_complete_graph() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let before = retained_state(&store)?;
    let (generation, plan) = Generation::new(&store, 0)?;
    assert_eq!(retained_state(&store)?, before);
    assert_eq!(plan.descriptors.len(), 9);
    assert_eq!(plan.capture(), &generation.capture);
    let inventory = generation.seal.inventory_including_seal(plan.seal())?;
    assert_eq!(inventory.object_count(), 9);
    assert_eq!(inventory.pages(), 1);
    assert_eq!(inventory.rows(), 1);
    assert_eq!(physical_rows(&store)?.len(), 0);
    let reservation = reserve(&store, "raw-graph", plan)?;
    assert_eq!(physical_rows(&store)?.len(), 0);
    assert_eq!(reservation.capture(), &generation.capture);
    assert_eq!(
        reservation.seal().physical_bytes(),
        bridge::Node::Seal(&generation.seal)
            .measure(&store)?
            .full_bytes()
    );
    let raw = row(&store, admission::key("raw-graph").as_bytes())?.ok_or("reservation")?;
    let expected_bytes = admission::HEADER_MARGIN
        .checked_add(u64::try_from(
            admission::key("raw-graph")
                .len()
                .checked_add(raw.len())
                .ok_or("reservation length")?,
        )?)
        .and_then(|sum| sum.checked_add(inventory.physical_bytes()))
        .ok_or("reservation byte total")?;
    assert_eq!(counters(&store)?, (expected_bytes, 9, 1));
    let reserved_header = header_bytes(&store)?;
    generation.stage(&store, &reservation)?;
    assert_eq!(physical_rows(&store)?.len(), 27);
    assert_eq!(physical_bytes(&store)?, inventory.physical_bytes());
    assert_eq!(header_bytes(&store)?, reserved_header);
    assert_eq!(counters(&store)?, (expected_bytes, 9, 1));
    let retried = load(&store, "raw-graph")?.ok_or("reservation retry")?;
    retried.require_plan(&generation.plan(&store)?)?;
    Ok(())
}

#[test]
fn ready_graph_reuse_has_zero_incremental_charge_and_positive_full_references() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let original_seal = plan.seal().clone();
    let reservation = reserve(&store, "first-graph", plan)?;
    generation.stage(&store, &reservation)?;
    let retained = retained_state(&store)?;
    let reused_plan = generation.plan(&store)?;
    reservation.require_plan(&reused_plan)?;
    assert_eq!(reused_plan.seal(), &original_seal);
    assert_eq!(retained_state(&store)?, retained);
    for descriptor in &reused_plan.descriptors {
        assert_eq!(
            (descriptor.incremental.bytes, descriptor.incremental.roots),
            (0, 0)
        );
        assert!(descriptor.object.physical_bytes() > u64::from(descriptor.object.bytes()));
        assert_eq!(
            descriptor.object.physical_bytes(),
            descriptor.physical.full_bytes()
        );
        assert_eq!(
            descriptor.object.root().as_str(),
            descriptor.physical.root()
        );
    }
    let before = counters(&store)?;
    let reused = reserve(&store, "second-graph", reused_plan)?;
    let raw = row(&store, admission::key("second-graph").as_bytes())?.ok_or("reuse reservation")?;
    let metadata = u64::try_from(
        admission::key("second-graph")
            .len()
            .checked_add(raw.len())
            .ok_or("metadata length")?,
    )?;
    assert_eq!(
        counters(&store)?,
        (
            before.0.checked_add(metadata).ok_or("metadata total")?,
            9,
            2
        )
    );
    let ready = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    generation.stage(&store, &reused)?;
    assert_eq!(retained_state(&store)?, ready);
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}
