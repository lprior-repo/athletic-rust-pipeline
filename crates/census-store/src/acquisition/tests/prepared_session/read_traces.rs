use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReadTrace {
    roots: u64,
    encoded: u64,
    receipts: u64,
    present: u64,
}

impl ReadTrace {
    fn from_budget(budget: &AcquisitionReadBudget) -> Self {
        let usage = budget.selection_usage();
        Self {
            roots: usage.roots(),
            encoded: usage.encoded_bytes(),
            receipts: usage.receipt_probes(),
            present: usage.present_receipts(),
        }
    }

    fn joined(self, effect: Self) -> TestResult<Self> {
        Ok(Self {
            roots: self.roots.checked_add(effect.roots).ok_or("joined roots")?,
            encoded: self
                .encoded
                .checked_add(effect.encoded)
                .ok_or("joined encoded")?,
            receipts: self
                .receipts
                .checked_add(effect.receipts)
                .ok_or("joined receipts")?,
            present: self
                .present
                .checked_add(effect.present)
                .ok_or("joined present")?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Winner {
    Unresolved,
    Retained,
}

struct TraceFixture {
    _directory: tempfile::TempDir,
    store: Store,
    owner: ResultsCaptureOwnerV1,
    target: PreparedRecoveryTarget,
    usage: AcquisitionSelectionUsage,
    selection: ReadTrace,
    effect: ReadTrace,
    body: Vec<u8>,
    body_path: std::path::PathBuf,
    metadata_path: std::path::PathBuf,
}

impl TraceFixture {
    fn pair(&self) -> TestResult<(Vec<u8>, Vec<u8>)> {
        Ok((
            std::fs::read(&self.body_path)?,
            std::fs::read(&self.metadata_path)?,
        ))
    }
}

fn expected_rows(store: &Store, exact: &AcquisitionWorkV1) -> TestResult<ReadTrace> {
    let snapshot = store.db.snapshot();
    let id = exact.id()?;
    let slot = exact.slot().id()?;
    let prefix = Store::journal_key(CaptureProtocolVersion::V2.phase(), &format!("work/{id}/"));
    let mut roots = 0_u64;
    let mut encoded = 0_u64;
    for guard in snapshot.prefix(&store.journal, &prefix).take(42) {
        let (key, value) = guard.into_inner()?;
        roots = roots.checked_add(1).ok_or("expected roots")?;
        encoded = encoded
            .checked_add(u64::try_from(
                key.len().checked_add(value.len()).ok_or("row bytes")?,
            )?)
            .ok_or("expected encoded")?;
    }
    let binding = Store::journal_key(
        CaptureProtocolVersion::V2.phase(),
        &format!("slot/{slot}/binding"),
    );
    let value = snapshot
        .get(&store.journal, &binding)?
        .ok_or("slot binding")?;
    roots = roots.checked_add(1).ok_or("binding root")?;
    encoded = encoded
        .checked_add(u64::try_from(
            binding
                .len()
                .checked_add(value.len())
                .ok_or("binding bytes")?,
        )?)
        .ok_or("binding encoded")?;
    let mut present = 0_u64;
    for ordinal in 1..=3 {
        let attempt = AcquisitionAttemptId::new(
            exact.slot().store_identity().clone(),
            id.clone(),
            AttemptOrdinal::new(ordinal)?,
        );
        let operation = read::terminal_operation(&attempt, CaptureProtocolVersion::V2);
        if let Some(value) = snapshot.get(&store.receipts, operation.as_bytes())? {
            present = present.checked_add(1).ok_or("expected present")?;
            encoded = encoded
                .checked_add(u64::try_from(
                    operation
                        .len()
                        .checked_add(value.len())
                        .ok_or("receipt bytes")?,
                )?)
                .ok_or("receipt encoded")?;
        }
    }
    Ok(ReadTrace {
        roots,
        encoded,
        receipts: 3,
        present,
    })
}

fn trace_fixture(winner: Winner) -> TestResult<TraceFixture> {
    let (directory, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    for status in [500, 429] {
        let observed = super::super::bounds::five_dispatch_response(
            &store,
            &writer,
            &owner,
            status,
            ConditionalRequestV1::Unconditional,
        )?;
        let (_prepared, _body) = super::super::bounds::prepare_response(&store, observed, status)?;
        let (target, usage) = selected(&store, &owner)?;
        let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
        let mut session = unresolved(
            store
                .begin_prepared_recovery(&owner, &target, &mut parent)?
                .read_backing()?,
        )?;
        let decision = session.missing_decision()?;
        let failed = session.close_missing(&decision)?;
        check!(eq; failed.receipt().appended, 0);
        check!(!failed.receipt().credited);
    }
    let observed = super::super::bounds::five_dispatch_response(
        &store,
        &writer,
        &owner,
        200,
        ConditionalRequestV1::Unconditional,
    )?;
    let (prepared, body) = super::super::bounds::prepare_response(&store, observed, 200)?;
    let (body_path, metadata_path) = publish(&store, &prepared, &body)?;
    let exact = work(&owner)?;
    let expected_selection = expected_rows(&store, &exact)?;
    check!(eq; (expected_selection.roots, expected_selection.receipts, expected_selection.present), (40, 3, 2));
    let mut selection_budget = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        store.read_existing_recovery_target(&owner, &exact, &mut selection_budget)?
    else {
        return Err("maximal J0 Prepared pin".into());
    };
    let selection = ReadTrace::from_budget(&selection_budget);
    check!(eq; selection, expected_selection);
    check!(eq; target.attempt().ordinal().get(), 3);
    if winner == Winner::Retained {
        let occurrence = store.finish_acquisition_capture(prepared)?;
        check!(eq; occurrence.attempt(), target.attempt());
    }
    let effect = expected_rows(&store, &exact)?;
    let expected = match winner {
        Winner::Unresolved => (40, 3, 2),
        Winner::Retained => (42, 3, 3),
    };
    check!(eq; (effect.roots, effect.receipts, effect.present), expected);
    Ok(TraceFixture {
        _directory: directory,
        store,
        owner,
        target,
        usage: selection_budget.selection_usage(),
        selection,
        effect,
        body,
        body_path,
        metadata_path,
    })
}

fn execute(
    fixture: &TraceFixture,
    parent: &mut PreparedRecoveryBudget,
) -> StoreResult<crate::acquisition::RetainedCaptureParts> {
    match fixture
        .store
        .begin_prepared_recovery(&fixture.owner, &fixture.target, parent)?
        .read_backing()?
    {
        PreparedRecoveryRead::Unresolved(session) => session.finish().map(|body| body.into_parts()),
        PreparedRecoveryRead::Retained(body) => Ok(body.into_parts()),
        _ => Err(CaptureError::PreparedTargetMismatch.into()),
    }
}

fn assert_effect(parent: &mut PreparedRecoveryBudget, expected: ReadTrace) -> TestResult {
    let observed = ReadTrace::from_budget(parent.reads());
    let counters = parent.reads().recovery_totals();
    check!(eq; observed, expected);
    check!(eq; (counters.roots.used, counters.encoded.used, counters.receipts.used), (observed.roots, observed.encoded, observed.receipts));
    Ok(())
}

fn report(
    fixture: &TraceFixture,
    parent: &mut PreparedRecoveryBudget,
    winner: Winner,
    dimension: &str,
    short: bool,
) -> TestResult {
    let effect = ReadTrace::from_budget(parent.reads());
    let joined = fixture.selection.joined(effect)?;
    eprintln!("core-read-trace winner={winner:?} dimension={dimension} short={short} J0={:?} J1={effect:?} joined={joined:?}", fixture.selection);
    Ok(())
}

#[test]
fn prepared_recovery_real_j0_j1_maximal_roots_80_79_and_82_81() -> TestResult {
    for winner in [Winner::Unresolved, Winner::Retained] {
        for short in [false, true] {
            let fixture = trace_fixture(winner)?;
            let joined = fixture.selection.joined(fixture.effect)?;
            check!(eq; joined.roots, if winner == Winner::Unresolved { 80 } else { 82 });
            check!(eq; (joined.receipts, joined.present), (6, if winner == Winner::Unresolved { 4 } else { 5 }));
            let mut parent = PreparedRecoveryBudget::admit(fixture.usage, 1)?;
            fixture
                .store
                .validate_prepared_recovery_owner(&fixture.owner, &mut parent)?;
            let gate = parent.reads().recovery_totals().admission.used;
            let remaining = joined
                .roots
                .checked_sub(u64::from(short))
                .and_then(|cap| cap.checked_sub(fixture.selection.roots))
                .ok_or("remaining roots")?;
            parent.reads().recovery_narrow_physical_reads(
                usize::try_from(remaining)?,
                usize::try_from(fixture.effect.encoded)?,
                3,
            );
            let before = rows(&fixture.store)?;
            let pair = fixture.pair()?;
            let result = execute(&fixture, &mut parent);
            if short {
                check!(matches!(
                    result,
                    Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
                        resource: "roots"
                    }))
                ));
                check!(eq; rows(&fixture.store)?, before);
                let observed = ReadTrace::from_budget(parent.reads());
                check!(eq; (observed.roots, observed.receipts, observed.present), (remaining, 0, 0));
                check!(eq; fixture.selection.joined(observed)?.roots, joined.roots.checked_sub(1).ok_or("short roots")?);
            } else {
                let parts = result?;
                check!(eq; parts.body.as_slice(), fixture.body.as_slice());
                check!(eq; parts.occurrence.attempt(), fixture.target.attempt());
                assert_effect(&mut parent, fixture.effect)?;
                check!(eq; fixture.selection.joined(ReadTrace::from_budget(parent.reads()))?, joined);
                if winner == Winner::Retained {
                    check!(eq; rows(&fixture.store)?, before);
                }
            }
            check!(eq; parent.reads().recovery_totals().admission.used, gate);
            check!(eq; fixture.pair()?, pair);
            report(&fixture, &mut parent, winner, "roots", short)?;
        }
    }
    Ok(())
}

#[test]
fn prepared_recovery_real_j0_j1_encoded_n_and_n_minus_one() -> TestResult {
    for winner in [Winner::Unresolved, Winner::Retained] {
        for short in [false, true] {
            let fixture = trace_fixture(winner)?;
            let joined = fixture.selection.joined(fixture.effect)?;
            let mut parent = PreparedRecoveryBudget::admit(fixture.usage, 1)?;
            fixture
                .store
                .validate_prepared_recovery_owner(&fixture.owner, &mut parent)?;
            let remaining = joined
                .encoded
                .checked_sub(u64::from(short))
                .and_then(|cap| cap.checked_sub(fixture.selection.encoded))
                .ok_or("remaining encoded")?;
            parent
                .reads()
                .recovery_narrow_physical_reads(42, usize::try_from(remaining)?, 3);
            let before = rows(&fixture.store)?;
            let pair = fixture.pair()?;
            let result = execute(&fixture, &mut parent);
            if short {
                check!(matches!(
                    result,
                    Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
                        resource: "encoded bytes"
                    }))
                ));
                check!(eq; rows(&fixture.store)?, before);
                let observed = ReadTrace::from_budget(parent.reads());
                check!(eq; observed.roots, fixture.effect.roots);
                check!(eq; (observed.receipts, observed.present), if winner == Winner::Unresolved { (2, 1) } else { (3, 2) });
                check!(observed.encoded <= remaining);
            } else {
                let parts = result?;
                check!(eq; parts.occurrence.attempt(), fixture.target.attempt());
                assert_effect(&mut parent, fixture.effect)?;
                check!(eq; fixture.selection.joined(ReadTrace::from_budget(parent.reads()))?, joined);
            }
            check!(eq; fixture.pair()?, pair);
            report(&fixture, &mut parent, winner, "encoded", short)?;
        }
    }
    Ok(())
}

#[test]
fn prepared_recovery_real_j0_j1_receipt_probes_6_and_5() -> TestResult {
    for winner in [Winner::Unresolved, Winner::Retained] {
        for short in [false, true] {
            let fixture = trace_fixture(winner)?;
            let mut parent = PreparedRecoveryBudget::admit(fixture.usage, 1)?;
            fixture
                .store
                .validate_prepared_recovery_owner(&fixture.owner, &mut parent)?;
            let joined = fixture.selection.joined(fixture.effect)?;
            let remaining = joined
                .receipts
                .checked_sub(u64::from(short))
                .and_then(|cap| cap.checked_sub(fixture.selection.receipts))
                .ok_or("remaining receipts")?;
            parent.reads().recovery_narrow_physical_reads(
                42,
                usize::try_from(fixture.effect.encoded)?,
                remaining,
            );
            let before = rows(&fixture.store)?;
            let pair = fixture.pair()?;
            let result = execute(&fixture, &mut parent);
            if short {
                check!(matches!(
                    result,
                    Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
                        resource: "receipt probes"
                    }))
                ));
                check!(eq; rows(&fixture.store)?, before);
                let observed = ReadTrace::from_budget(parent.reads());
                check!(eq; (observed.roots, observed.receipts, observed.present), (fixture.effect.roots, 2, 2));
                check!(eq; fixture.selection.joined(observed)?.receipts, 5);
            } else {
                let parts = result?;
                check!(eq; parts.occurrence.attempt(), fixture.target.attempt());
                assert_effect(&mut parent, fixture.effect)?;
                check!(eq; fixture.selection.joined(ReadTrace::from_budget(parent.reads()))?.receipts, 6);
            }
            check!(eq; fixture.pair()?, pair);
            report(&fixture, &mut parent, winner, "receipts", short)?;
        }
    }
    Ok(())
}
