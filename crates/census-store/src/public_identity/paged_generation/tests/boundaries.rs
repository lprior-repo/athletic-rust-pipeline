use super::*;
use crate::StoreError;

#[test]
fn exact_key_value_fragment_boundaries_and_one_over() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let overhead = crate::keys::observation_key(crate::Table::IdentityVerdicts, "", 0).len();
    let id = "a".repeat(
        MAX_RAW_KEY_BYTES
            .checked_sub(overhead)
            .ok_or("key overhead")?,
    );
    let admitted = row(&binding, &id, MAX_RAW_VALUE_BYTES)?;
    check!(eq;admitted.physical_key().len(), MAX_RAW_KEY_BYTES);
    check!(eq;admitted.raw_value().len(), MAX_RAW_VALUE_BYTES);
    check!(row(&binding, &format!("{id}a"), MAX_RAW_VALUE_BYTES).is_err());
    check!(row(&binding, "a", MAX_RAW_VALUE_BYTES + 1).is_err());
    for length in [
        RAW_FRAGMENT_BYTES,
        RAW_FRAGMENT_BYTES + 1,
        MAX_RAW_VALUE_BYTES,
    ] {
        let page = RawPageV2::new(
            &binding,
            RawPagePositionV2::new(0, 0)?,
            vec![row(&binding, "a", length)?],
        )?;
        let bytes = page.encode_jsonl()?;
        check!(eq;RawPageV2::decode(&bytes, &object(&bytes)?, &binding)?, page);
    }
    Ok(())
}

#[test]
fn exact_512_rows_and_one_over() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let rows = (0..MAX_PAGE_ROWS)
        .map(|i| row(&binding, &format!("{i:04}"), 0))
        .collect::<TestResult<Vec<_>>>()?;
    let page = RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, rows.clone())?;
    let bytes = page.encode_jsonl()?;
    check!(eq;
        RawPageV2::decode(&bytes, &object(&bytes)?, &binding)?
            .rows()
            .len(),
        512
    );
    let mut excess = rows;
    excess.push(row(&binding, "9999", 0)?);
    check!(RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, excess).is_err());
    Ok(())
}

#[test]
fn exact_sixteen_mib_raw_page_and_one_over() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let rows = (0..16)
        .map(|i| {
            let id = format!("{i:04}");
            let key = crate::keys::observation_key(crate::Table::IdentityVerdicts, &id, 0);
            row(
                &binding,
                &id,
                MAX_RAW_VALUE_BYTES
                    .checked_sub(key.len())
                    .ok_or("row length")?,
            )
        })
        .collect::<TestResult<Vec<_>>>()?;
    let page = RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, rows.clone())?;
    check!(eq;page.header.raw_bytes, MAX_PAGE_RAW_BYTES);
    let bytes = page.encode_jsonl()?;
    check!(bytes.len() <= usize::try_from(MAX_PAGE_JSONL_BYTES)?);
    check!(eq;RawPageV2::decode(&bytes, &object(&bytes)?, &binding)?, page);
    let mut too_large = rows;
    let first = too_large.first_mut().ok_or("row")?;
    let mut value = first.raw_value().to_vec();
    value.push(b' ');
    *first = RawPhysicalRowV2::new(&binding, first.physical_key(), &value)?;
    check!(RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, too_large).is_err());
    Ok(())
}

#[test]
fn object_and_record_limits_precede_json_decode() -> TestResult {
    let (binding, _) = page()?;
    let huge = vec![b'x'; usize::try_from(MAX_PAGE_JSONL_BYTES)? + 1];
    check!(ObjectRefV2::new(
        DigestV2::of_bytes(&huge),
        u32::try_from(huge.len())?,
        1,
        u64::try_from(huge.len())?
    )
    .is_err());
    let mut line = vec![b' '; MAX_RECORD_BYTES];
    line.push(b'\n');
    check!(
        matches!(RawPageV2::decode(&line, &object(&line)?, &binding),
        Err(StoreError::Refused { detail }) if detail == "raw public identity generation V2: record length or missing LF")
    );
    check!(ObjectRefV2::new(DigestV2::of_bytes(b"x"), 1, MAX_PAGE_RECORDS + 1, 1).is_err());
    check!(ObjectRefV2::new(DigestV2::of_bytes(b"x"), 2, 1, 1).is_err());
    check!(RawPagePositionV2::new(MAX_TABLE_PAGES, 0).is_err());
    check!(RawPagePositionV2::new(1, u64::MAX).is_err());
    check!(page::validate_position(MAX_TABLE_PAGES - 1, MAX_TABLE_ROWS - 1, 2, 10).is_err());
    Ok(())
}

#[test]
fn generation_inventory_caps_are_inclusive_and_addition_is_checked() -> TestResult {
    let maximum = wire::Inventory {
        pages: MAX_GENERATION_PAGES,
        rows: MAX_GENERATION_ROWS,
        raw_bytes: MAX_GENERATION_RAW_BYTES,
        object_count: MAX_GENERATION_OBJECTS,
        jsonl_bytes: MAX_GENERATION_JSONL_BYTES,
        physical_bytes: MAX_GENERATION_PHYSICAL_BYTES,
    };
    check!(InventoryV2::from_wire(maximum).is_ok());
    for excess in [
        wire::Inventory {
            pages: maximum.pages + 1,
            ..maximum
        },
        wire::Inventory {
            rows: maximum.rows + 1,
            ..maximum
        },
        wire::Inventory {
            raw_bytes: maximum.raw_bytes + 1,
            ..maximum
        },
        wire::Inventory {
            object_count: maximum.object_count + 1,
            ..maximum
        },
        wire::Inventory {
            jsonl_bytes: maximum.jsonl_bytes + 1,
            ..maximum
        },
        wire::Inventory {
            physical_bytes: maximum.physical_bytes + 1,
            ..maximum
        },
    ] {
        check!(InventoryV2::from_wire(excess).is_err());
    }
    let corrupt = InventoryV2(wire::Inventory {
        rows: u64::MAX,
        ..wire::Inventory::default()
    });
    let one = InventoryV2(wire::Inventory {
        rows: 1,
        ..wire::Inventory::default()
    });
    check!(matches!(
        corrupt.plus(one),
        Err(StoreError::CounterOverflow)
    ));
    let table = InventoryV2(wire::Inventory {
        pages: MAX_TABLE_PAGES,
        rows: MAX_TABLE_ROWS,
        ..wire::Inventory::default()
    });
    check!(table.for_table().is_ok());
    check!(InventoryV2(wire::Inventory {
        pages: MAX_TABLE_PAGES + 1,
        ..table.0
    })
    .for_table()
    .is_err());
    check!(InventoryV2(wire::Inventory {
        rows: MAX_TABLE_ROWS + 1,
        ..table.0
    })
    .for_table()
    .is_err());
    Ok(())
}

#[test]
fn aggregate_physical_bytes_cannot_be_less_than_encoded_bytes() -> TestResult {
    check!(InventoryV2::from_wire(wire::Inventory {
        jsonl_bytes: 2,
        physical_bytes: 1,
        ..wire::Inventory::default()
    })
    .is_err());
    check!(InventoryV2::from_wire(wire::Inventory {
        jsonl_bytes: 2,
        physical_bytes: 2,
        ..wire::Inventory::default()
    })
    .is_ok());
    check!(page::validate_position(0, 0, 2, 3).is_err());
    Ok(())
}

#[test]
fn directory_and_seal_wire_ceilings_are_checked_before_payload_parse() -> TestResult {
    let root = DigestV2::of_bytes(b"boundary");
    for limit in [limits::ObjectLimits::DIRECTORY, limits::ObjectLimits::SEAL] {
        let raw = wire::ObjectRef {
            root: root.clone(),
            bytes: limit.bytes,
            records: limit.records,
            physical_bytes: u64::from(limit.bytes),
        };
        check!(ObjectRefV2::from_wire(raw.clone(), limit).is_ok());
        check!(ObjectRefV2::from_wire(
            wire::ObjectRef {
                bytes: limit.bytes + 1,
                physical_bytes: u64::from(limit.bytes) + 1,
                ..raw.clone()
            },
            limit
        )
        .is_err());
        check!(ObjectRefV2::from_wire(
            wire::ObjectRef {
                records: limit.records + 1,
                ..raw
            },
            limit
        )
        .is_err());
    }
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let bytes = vec![b'x'; usize::try_from(MAX_DIRECTORY_JSONL_BYTES)? + 1];
    let reference = ObjectRefV2::new(
        DigestV2::of_bytes(&bytes),
        u32::try_from(bytes.len())?,
        1,
        u64::try_from(bytes.len())?,
    )?;
    check!(
        matches!(RawPageBlockV2::decode(&bytes, &reference, &binding),
        Err(StoreError::Refused { detail }) if detail == "raw public identity generation V2: object reference bounds")
    );
    let bytes = vec![b'x'; usize::try_from(MAX_SEAL_JSONL_BYTES)? + 1];
    let reference = ObjectRefV2::new(
        DigestV2::of_bytes(&bytes),
        u32::try_from(bytes.len())?,
        1,
        u64::try_from(bytes.len())?,
    )?;
    check!(
        matches!(RawGenerationSealV2::decode(&bytes, &reference, &capture()?),
        Err(StoreError::Refused { detail }) if detail == "raw public identity generation V2: object reference bounds")
    );
    Ok(())
}
