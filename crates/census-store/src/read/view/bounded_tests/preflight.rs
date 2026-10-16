use super::*;

fn budget() -> ReadBudget {
    ReadBudget {
        rows: 1,
        bytes: 8,
        record_bytes: 4,
    }
}

#[test]
fn bounded_value_preflight_accepts_exact_record_and_cumulative_byte_limits() -> TestResult {
    let mut counts = ReadCounts {
        encoded_bytes: 2,
        ..ReadCounts::default()
    };
    let mut loads = 0;
    let value = read_admitted(2, Some(4), &mut counts, budget(), || {
        loads += 1;
        Ok(Some(fjall::UserValue::from(&b"null"[..])))
    })?;
    check!(eq; value.as_ref(), b"null");
    check!(eq; loads, 1);
    check!(eq; counts.encoded_bytes, 8);
    check!(eq; counts.decoded_rows, 0);
    Ok(())
}

#[test]
fn bounded_value_preflight_refuses_record_bytes_before_loading_value() -> TestResult {
    let mut counts = ReadCounts::default();
    let mut loaded = false;
    let result = read_admitted(2, Some(5), &mut counts, budget(), || {
        loaded = true;
        Ok(None::<fjall::UserValue>)
    });
    check!(matches!(
        result,
        Err(StoreError::RowTooLarge { bytes: 5, max: 4 })
    ));
    check!(!loaded);
    check!(eq; counts, ReadCounts::default());
    Ok(())
}

#[test]
fn bounded_value_preflight_charges_key_and_prior_bytes_before_loading_value() -> TestResult {
    let mut counts = ReadCounts {
        encoded_bytes: 3,
        ..ReadCounts::default()
    };
    let mut loaded = false;
    let result = read_admitted(2, Some(4), &mut counts, budget(), || {
        loaded = true;
        Ok(None::<fjall::UserValue>)
    });
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "capture encoded bytes",
            needed: 9,
            available: 8
        })
    ));
    check!(!loaded);
    check!(eq; counts.encoded_bytes, 3);
    Ok(())
}

#[test]
fn bounded_value_preflight_refuses_missing_size_before_loading_value() -> TestResult {
    let mut loaded = false;
    let result = read_admitted(2, None, &mut ReadCounts::default(), budget(), || {
        loaded = true;
        Ok(None::<fjall::UserValue>)
    });
    check!(matches!(result, Err(StoreError::Invariant { .. })));
    check!(!loaded);
    Ok(())
}

#[test]
fn bounded_value_preflight_refuses_missing_or_changed_admitted_values() -> TestResult {
    for (value, expected) in [
        (None, "admitted bounded observation is missing"),
        (
            Some(fjall::UserValue::from(&b"false"[..])),
            "admitted bounded observation size changed",
        ),
        (
            Some(fjall::UserValue::from(&b"0"[..])),
            "admitted bounded observation size changed",
        ),
    ] {
        let result = read_admitted(2, Some(4), &mut ReadCounts::default(), budget(), || {
            Ok(value)
        });
        check!(matches!(result, Err(StoreError::Invariant { detail }) if detail == expected));
    }
    Ok(())
}

#[test]
fn bounded_observation_preflight_uses_the_pinned_size_and_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let row = RetainedConflict::new("fixture", "one", "subject", "before replacement");
    store.append(Table::Schools, &row)?;
    let held = store.snapshot();
    let key = store
        .entities
        .prefix(crate::keys::observation_prefix(Table::Schools))
        .next()
        .ok_or("missing fixture row")?
        .key()?;
    let raw = serde_json::to_vec(&row)?;
    let bytes = u64::try_from(key.len().checked_add(raw.len()).ok_or("fixture overflow")?)?;
    let limits = ReadBudget {
        rows: 1,
        bytes,
        record_bytes: raw.len(),
    };
    let oversized = vec![b'!'; raw.len().checked_add(1).ok_or("fixture overflow")?];
    store.entities.insert(key.as_ref(), &oversized)?;
    let mut seen = Vec::new();
    let mut charges = Vec::new();
    let counts = held.for_each_observation_bounded(
        Table::Schools,
        limits,
        |value: RetainedConflict, charged| {
            charges.push(charged);
            seen.push(value);
            Ok(())
        },
    )?;
    check!(eq; charges, vec![bytes]);
    check!(eq; seen, vec![row]);
    check!(eq; counts, ReadCounts {
        key_visits: 1,
        decoded_rows: 1,
        encoded_bytes: bytes,
        merged_rows: 0,
    });
    let mut visited = false;
    let result = store
        .snapshot()
        .for_each_observation_bounded::<RetainedConflict>(Table::Schools, limits, |_, _| {
            visited = true;
            Ok(())
        });
    check!(
        matches!(result, Err(StoreError::RowTooLarge { bytes, max }) if bytes == oversized.len() && max == raw.len())
    );
    check!(!visited);
    Ok(())
}
