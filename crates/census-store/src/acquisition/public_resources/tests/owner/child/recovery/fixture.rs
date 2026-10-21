use super::*;

pub(super) struct PreparedChild {
    pub(super) f: Fixture,
    pub(super) owner: PublicResourceCaptureOwnerV1,
    pub(super) original: AcquisitionOccurrenceRefV1,
    pub(super) work: AcquisitionWorkV1,
    pub(super) attempt: AcquisitionAttemptId,
    pub(super) input: PreparedCaptureInputV1,
    pub(super) body: Vec<u8>,
}

impl PreparedChild {
    pub(super) fn new() -> TestResult<Self> {
        let f = Fixture::new()?;
        let (owner, original) = retained(&f)?;
        let assessed = assessment(&f.store, &owner, &original)?;
        let owned = f
            .store
            .register_public_roster_child(f.store.prepare_public_roster_child(&assessed)?)?;
        let work = owned.work().clone();
        let (prepared, body) = observed::prepare_work(
            &f.store,
            owned,
            b"<html><body>CONTROLLED_PROTOCOL_ROSTER</body></html>".to_vec(),
        )?;
        let attempt = prepared.attempt_id().clone();
        let input = prepared.prepared().clone();
        observed::publish(&f.store, &input, &body)?;
        drop(prepared);
        Ok(Self {
            f,
            owner,
            original,
            work,
            attempt,
            input,
            body,
        })
    }

    pub(super) fn cold_roundtrip(self) -> TestResult<Self> {
        let before = Witness::capture(&self.f.store)?;
        let Fixture {
            directory,
            store,
            writer,
            binding,
        } = self.f;
        let root = directory.path().join("store");
        store.flush()?;
        drop((store, writer));
        for current in [true, false] {
            let store = cold_open(&root, current)?;
            check!(eq; Witness::capture(&store)?, before);
            drop(store);
        }
        let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
        Ok(Self {
            f: Fixture {
                directory,
                store,
                writer,
                binding,
            },
            owner,
            ..self
        })
    }
}

pub(super) fn selected(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    work: &AcquisitionWorkV1,
) -> TestResult<(PreparedRecoveryTarget, AcquisitionSelectionUsage)> {
    let mut budget = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        store.read_existing_recovery_target(owner, work, &mut budget)?
    else {
        return Err("controlled child prepared target absent".into());
    };
    check!(budget.public_parent_usage().rows() > 0);
    Ok((target, budget.selection_usage()))
}

pub(super) fn core_usage(
    store: &Store,
    target: &PreparedRecoveryTarget,
) -> TestResult<AcquisitionSelectionUsage> {
    let mut budget = AcquisitionReadBudget::for_work();
    crate::acquisition::read::read_work(
        &store.db.snapshot(),
        &store.journal,
        &store.receipts,
        target.attempt().work_id(),
        &mut budget,
        CaptureProtocolVersion::V2,
    )?;
    Ok(budget.selection_usage())
}

pub(super) fn execute(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    target: &PreparedRecoveryTarget,
    budget: &mut PreparedRecoveryBudget,
) -> crate::StoreResult<RetainedCaptureParts> {
    match store
        .begin_prepared_recovery(owner, target, budget)?
        .read_backing()?
    {
        PreparedRecoveryRead::Unresolved(session) => session.finish().map(|body| body.into_parts()),
        PreparedRecoveryRead::Retained(body) => Ok(body.into_parts()),
        _ => Err(CaptureError::PreparedTargetMismatch.into()),
    }
}

pub(super) fn damage_parent(
    store: &Store,
    original: &AcquisitionOccurrenceRefV1,
    fault: &str,
) -> TestResult {
    match fault {
        "terminal-receipt" => {
            let operation = crate::acquisition::read::terminal_operation(
                original.attempt(),
                CaptureProtocolVersion::V2,
            );
            store.receipts.remove(operation.as_bytes())?;
        }
        "result-reference" => {
            let key = Store::journal_key(
                CaptureProtocolVersion::V2.phase(),
                &format!("work/{}/result", original.attempt().work_id()),
            );
            let held = store.journal.get(&key)?.ok_or("parent result envelope")?;
            let mut envelope: serde_json::Value = serde_json::from_slice(&held)?;
            let payload = envelope.get_mut("payload").ok_or("parent result payload")?;
            let occurrence = payload
                .get_mut("occurrence")
                .ok_or("parent result occurrence")?;
            *occurrence = serde_json::to_value(AcquisitionOccurrenceRefV1::new(
                original.attempt().clone(),
                different_digest(original.terminal_digest())?,
            ))?;
            store.journal.insert(key, serde_json::to_vec(&envelope)?)?;
        }
        _ => return Err("unknown parent protocol fault".into()),
    }
    store.flush()?;
    Ok(())
}
