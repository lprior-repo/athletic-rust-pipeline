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
    check!(eq;retained_state(&store)?, before);
    check!(eq;plan.descriptors.len(), 9);
    check!(eq;&plan.capture, &generation.capture);
    let inventory = generation.seal.inventory_including_seal(&plan.seal)?;
    check!(eq;inventory.object_count(), 9);
    check!(eq;inventory.pages(), 1);
    check!(eq;inventory.rows(), 1);
    check!(eq;physical_rows(&store)?.len(), 0);
    let reservation = reserve(&store, "raw-graph", plan)?;
    check!(eq;physical_rows(&store)?.len(), 0);
    check!(eq;reservation.capture(), &generation.capture);
    check!(eq;
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
    check!(eq;counters(&store)?, (expected_bytes, 9, 1));
    let reserved_header = header_bytes(&store)?;
    generation.stage(&store, &reservation)?;
    check!(eq;physical_rows(&store)?.len(), 27);
    check!(eq;physical_bytes(&store)?, inventory.physical_bytes());
    check!(eq;header_bytes(&store)?, reserved_header);
    check!(eq;counters(&store)?, (expected_bytes, 9, 1));
    let retried = load(&store, "raw-graph")?.ok_or("reservation retry")?;
    retried.require_plan(&generation.plan(&store)?)?;
    Ok(())
}

#[test]
fn ready_graph_reuse_has_zero_incremental_charge_and_positive_full_references() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (generation, plan) = Generation::new(&store, 0)?;
    let original_seal = plan.seal.clone();
    let reservation = reserve(&store, "first-graph", plan)?;
    generation.stage(&store, &reservation)?;
    let retained = retained_state(&store)?;
    let reused_plan = generation.plan(&store)?;
    reservation.require_plan(&reused_plan)?;
    check!(eq;&reused_plan.seal, &original_seal);
    check!(eq;retained_state(&store)?, retained);
    for descriptor in &reused_plan.descriptors {
        check!(eq;
            (descriptor.incremental.bytes, descriptor.incremental.roots),
            (0, 0)
        );
        check!(descriptor.object.physical_bytes() > u64::from(descriptor.object.bytes()));
        check!(eq;
            descriptor.object.physical_bytes(),
            descriptor.physical.full_bytes()
        );
        check!(eq;
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
    check!(eq;
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
    check!(eq;retained_state(&store)?, ready);
    check!(eq;store.snapshot().sequence(), sequence);
    Ok(())
}
