use super::super::rendezvous::{self, Phase};
use super::*;

type PhysicalRows = Vec<(Vec<u8>, Vec<u8>)>;
type AuthorityState = (Vec<u8>, PhysicalRows, PhysicalRows);

pub(super) fn authority_state(
    store: &Store,
    writer: &PublicIdentityWriterV1,
) -> TestResult<AuthorityState> {
    let head = serde_json::to_vec(&writer.head(store)?)?;
    let receipts = store
        .receipts
        .iter()
        .take(2_049)
        .map(|entry| {
            let (key, value) = entry.into_inner()?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect::<TestResult<Vec<_>>>()?;
    if receipts.len() > 2_048 {
        return Err("authority receipt test bound".into());
    }
    let snapshot = store.db.snapshot();
    let meta = [
        crate::public_identity::DECLARATION,
        crate::public_identity::HEAD,
        crate::public_identity::HISTORY,
    ]
    .into_iter()
    .map(|key| {
        let value = crate::public_identity::codec::load(
            &snapshot,
            &store.meta,
            key,
            crate::public_identity::MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
        )?
        .ok_or("authority metadata")?;
        Ok((key.as_bytes().to_vec(), value.to_vec()))
    })
    .collect::<TestResult<PhysicalRows>>()?;
    Ok((head, meta, receipts))
}

pub(super) fn admission_header(store: &Store) -> TestResult<Vec<u8>> {
    Ok(crate::public_identity::codec::load(
        &store.db.snapshot(),
        &store.journal,
        "manual-context-admission-v1\0header",
        4096,
    )?
    .ok_or("charge header")?
    .to_vec())
}

fn reservation(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    id: &RawGenerationOperationIdV2,
) -> TestResult<GraphReservation> {
    let _guard = store.lock_appends();
    Ok(load_graph(store, writer.store_identity(), id)?.ok_or("reservation absent at phase")?)
}

fn mutation(store: &Store) -> TestResult {
    store.append(Table::Athletes, &subject("concurrent new input", "99901")?)?;
    Ok(())
}

fn require_moved(result: StoreResult<RawGenerationCaptureOutcomeV2>) -> TestResult {
    assert!(
        matches!(&result, Err(StoreError::Refused { detail })
        if detail == "raw public identity generation V2: capture input lineage or revisions moved"),
        "expected moved-input refusal, observed {result:?}"
    );
    Ok(())
}

#[test]
fn public_capture_concurrent_source_mutation_after_reservation_refuses_before_seal() -> TestResult {
    let (_directory, store, writer) = seeded_and_reopened(513)?;
    let authority = authority_state(&store, &writer)?;
    let id = operation("public-before-seal")?;
    let (gate, controller) = rendezvous::pair(Phase::AfterReservation);
    std::thread::scope(|scope| -> TestResult {
        let worker = scope.spawn(|| {
            rendezvous::with_gate(gate, || writer.capture_raw_generation_v2(&store, &id))
        });
        controller.wait()?;
        let reserved = reservation(&store, &writer, &id)?;
        let charge = admission_header(&store)?;
        assert!(!reserved.is_complete(&store)?);
        mutation(&store)?;
        controller.resume()?;
        require_moved(worker.join().map_err(|_| "capture worker panicked")?)?;
        assert!(!reserved.is_complete(&store)?);
        let sequence = store.snapshot().sequence();
        require_moved(writer.capture_raw_generation_v2(&store, &id))?;
        assert_eq!(store.snapshot().sequence(), sequence);
        assert_eq!(reservation(&store, &writer, &id)?.seal(), reserved.seal());
        assert_eq!(authority_state(&store, &writer)?, authority);
        assert_eq!(admission_header(&store)?, charge);
        Ok(())
    })
}

#[test]
fn public_capture_concurrent_source_mutation_after_seal_refuses_final_return() -> TestResult {
    let (_directory, store, writer) = seeded_and_reopened(513)?;
    let authority = authority_state(&store, &writer)?;
    let id = operation("public-after-seal")?;
    let (gate, controller) = rendezvous::pair(Phase::AfterSeal);
    std::thread::scope(|scope| -> TestResult {
        let worker = scope.spawn(|| {
            rendezvous::with_gate(gate, || writer.capture_raw_generation_v2(&store, &id))
        });
        controller.wait()?;
        let reserved = reservation(&store, &writer, &id)?;
        let charge = admission_header(&store)?;
        assert!(reserved.is_complete(&store)?);
        mutation(&store)?;
        let sequence = store.snapshot().sequence();
        controller.resume()?;
        require_moved(worker.join().map_err(|_| "capture worker panicked")?)?;
        assert_eq!(store.snapshot().sequence(), sequence);
        let replay = writer.capture_raw_generation_v2(&store, &id)?;
        assert!(matches!(
            &replay,
            RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
        ));
        assert_eq!(replay.retained().seal(), reserved.seal());
        assert_eq!(retained_rows(&store, replay.retained())?, 513);
        assert_eq!(store.snapshot().sequence(), sequence);
        assert_eq!(authority_state(&store, &writer)?, authority);
        assert_eq!(admission_header(&store)?, charge);
        Ok(())
    })
}

#[test]
fn public_same_operation_overlap_reuses_one_charge_and_one_retained_graph() -> TestResult {
    let (_directory, store, writer) = seeded_and_reopened(513)?;
    let authority = authority_state(&store, &writer)?;
    let id = operation("public-overlap")?;
    let (gate, controller) = rendezvous::pair(Phase::AfterReservation);
    std::thread::scope(|scope| -> TestResult {
        let worker = scope.spawn(|| {
            rendezvous::with_gate(gate, || writer.capture_raw_generation_v2(&store, &id))
        });
        controller.wait()?;
        let reserved = reservation(&store, &writer, &id)?;
        let charge = admission_header(&store)?;
        let second = writer.capture_raw_generation_v2(&store, &id)?;
        assert!(matches!(
            &second,
            RawGenerationCaptureOutcomeV2::Captured(_)
        ));
        assert_eq!(second.retained().seal(), reserved.seal());
        let sequence = store.snapshot().sequence();
        controller.resume()?;
        let first = worker.join().map_err(|_| "capture worker panicked")??;
        assert!(matches!(&first, RawGenerationCaptureOutcomeV2::Captured(_)));
        assert_eq!(first.retained(), second.retained());
        assert_eq!(store.snapshot().sequence(), sequence);
        assert!(reservation(&store, &writer, &id)?.is_complete(&store)?);
        assert_eq!(retained_rows(&store, first.retained())?, 513);
        assert_eq!(authority_state(&store, &writer)?, authority);
        assert_eq!(admission_header(&store)?, charge);
        Ok(())
    })
}
