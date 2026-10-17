use super::*;
use census_store::{Store, Table};

mod observations;
mod refusals;

pub(in crate::participation_evidence::recorded) fn verify_genuine(store: &Store) -> TestResult {
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = receipts(store)?;
    let result = verify(store);
    let tables_observed: TestResult = (|| {
        check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
        Ok(())
    })();
    let receipts_observed: TestResult = (|| {
        check!(eq; self::receipts(store)?, receipts);
        Ok(())
    })();
    match result {
        Ok(()) => {
            tables_observed?;
            receipts_observed
        }
        Err(error) => {
            eprintln!(
                "discovery_failure_preservation tables={} receipts={}",
                tables_observed.is_ok(),
                receipts_observed.is_ok()
            );
            Err(error)
        }
    }
}

fn verify(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted genuine target absent")?;
    let mut live_grant = AttachmentReplayGrantV1::new();
    let live = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted,
        &mut live_grant,
    )?;
    let plan = discover_current_html_results(&live, &mut live_grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    check!(plan.diagnostic(0).is_none());
    let observed = plan.with_group(0, &mut live_grant, |document, _| {
        Ok(observations::observe(document))
    })??;
    drop(plan);
    drop(live);
    let mut freeze_grant = AttachmentReplayGrantV1::new();
    let capture = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted,
        &mut freeze_grant,
    )?;
    let bundle = capture.freeze_closure(&mut freeze_grant)?;
    drop(capture);
    let mut frozen_grant = AttachmentReplayGrantV1::new();
    let frozen = census_store::replay_frozen_evidence_subject_attachment_references_v1(
        &bundle,
        &accepted,
        &mut frozen_grant,
    )?;
    check!(eq; frozen.store_identity(), accepted.store_identity());
    check!(eq; frozen.snapshot_sequence(), snapshot.sequence());
    check!(eq; frozen.accepted_public_head(), accepted.head());
    let plan = discover_frozen_html_results(&frozen, &mut frozen_grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    check!(plan.diagnostic(0).is_none());
    let restored = plan.with_group(0, &mut frozen_grant, |document, _| {
        Ok(observations::observe(document))
    })??;
    check!(eq; restored, observed);
    drop(plan);
    drop(frozen);
    refusals::verify(store)
}

fn receipts(store: &Store) -> TestResult<Vec<census_store::Receipt>> {
    let mut rows = Vec::new();
    let count = store.for_each_receipt(64, |receipt| {
        rows.push(receipt);
        Ok(())
    })?;
    check!(eq; count, rows.len());
    rows.sort_by(|left, right| left.operation.cmp(&right.operation));
    Ok(rows)
}
