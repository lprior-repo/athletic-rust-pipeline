use super::*;

#[test]
fn incomplete_nonrequested_table_prevents_verified_handle() -> TestResult {
    let (_dir, store, retained) = captured(1)?;
    let mut frozen = FrozenFixture::capture(&store.snapshot(), &retained)?;
    let seal = frozen.read_seal(retained.seal(), retained.binding())?;
    let reference = seal
        .tables()
        .iter()
        .find(|value| value.table() == AuthorityTableV2::RelayResults)
        .ok_or("relay table")?;
    frozen
        .objects
        .remove(reference.object().root().as_str())
        .ok_or("removed object")?;
    match VerifiedRawGraph::verify(&frozen, retained.binding(), retained.seal()) {
        Err(StoreError::Refused { detail }) => {
            check!(eq; detail, "raw public identity generation V2: frozen fixture object absent")
        }
        _ => return Err("missing nonrequested table must prevent verification".into()),
    }
    Ok(())
}

#[test]
fn immutable_source_rechecks_hash_and_retained_capture_identity() -> TestResult {
    let (_dir, store, retained) = captured(1)?;
    let mut frozen = FrozenFixture::capture(&store.snapshot(), &retained)?;
    let (_other_dir, _other_store, other) = captured(1)?;
    match VerifiedRawGraph::verify(&frozen, other.binding(), retained.seal()) {
        Err(StoreError::Refused { detail }) => {
            check!(eq; detail, "raw public identity generation V2: frozen fixture capture identity")
        }
        _ => return Err("foreign capture accepted".into()),
    }
    let (_, bytes) = frozen
        .objects
        .get_mut(retained.seal().root().as_str())
        .ok_or("seal")?;
    *bytes.first_mut().ok_or("byte")? = b'[';
    match VerifiedRawGraph::verify(&frozen, retained.binding(), retained.seal()) {
        Err(StoreError::Refused { detail }) => {
            check!(eq; detail, "raw public identity generation V2: object digest or byte length")
        }
        _ => return Err("corrupt immutable bytes accepted".into()),
    }
    Ok(())
}

#[test]
fn unchanged_scan_caps_are_admitted_before_consumer_callbacks() -> TestResult {
    let (_dir, store, retained) = captured(2)?;
    let snapshot = store.snapshot();
    let graph = VerifiedRawGraph::verify(&snapshot, retained.binding(), retained.seal())?;
    let inventory = graph.table_inventory(AuthorityTableV2::Athletes)?;
    let mut called = false;
    let rows = ReadBudget {
        rows: 1,
        ..budget()
    };
    let result =
        graph.for_each_observation::<CanonicalAthlete>(AuthorityTableV2::Athletes, rows, |_, _| {
            called = true;
            Ok(())
        });
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 2,
            available: 1
        })
    ));
    check!(!called);
    let bytes = ReadBudget {
        bytes: inventory.raw_bytes().checked_sub(1).ok_or("bytes")?,
        ..budget()
    };
    let result =
        graph.for_each_merged::<CanonicalAthlete>(AuthorityTableV2::Athletes, bytes, |_, _| {
            called = true;
            Ok(())
        });
    check!(
        matches!(result, Err(StoreError::BudgetExceeded { what: "capture encoded bytes", needed, available }) if needed == inventory.raw_bytes() && available == bytes.bytes)
    );
    check!(!called);
    let record = ReadBudget {
        record_bytes: 0,
        ..budget()
    };
    let result = graph.for_each_observation::<CanonicalAthlete>(
        AuthorityTableV2::Athletes,
        record,
        |_, _| {
            called = true;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::RowTooLarge { bytes, max: 0 }) if bytes > 0));
    check!(!called);
    let exact = ReadBudget {
        rows: 2,
        bytes: inventory.raw_bytes(),
        ..budget()
    };
    let counts = graph.for_each_observation::<CanonicalAthlete>(
        AuthorityTableV2::Athletes,
        exact,
        |_, _| {
            called = true;
            Ok(())
        },
    )?;
    check!(called);
    check!(eq; counts.decoded_rows, 2);
    check!(eq; counts.encoded_bytes, inventory.raw_bytes());
    Ok(())
}

#[test]
fn consumer_error_stops_scan_without_additional_callbacks() -> TestResult {
    let (_dir, store, retained) = captured(2)?;
    let snapshot = store.snapshot();
    let graph = VerifiedRawGraph::verify(&snapshot, retained.binding(), retained.seal())?;
    let mut calls = 0usize;
    let result = graph.for_each_observation::<CanonicalAthlete>(
        AuthorityTableV2::Athletes,
        budget(),
        |_, _| {
            calls += 1;
            Err(StoreError::Refused {
                detail: "consumer refused".into(),
            })
        },
    );
    check!(matches!(result, Err(StoreError::Refused { detail }) if detail == "consumer refused"));
    check!(eq; calls, 1);
    Ok(())
}
