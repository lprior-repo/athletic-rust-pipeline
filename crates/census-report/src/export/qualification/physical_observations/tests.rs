use super::*;
use census_domain::model::{CanonicalAthlete, ProfileDate, ProfileId};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn physical_admission_covers_the_original_refusal_and_exact_sixteen_gib() -> TestResult {
    for bytes in [4_294_968_825, 16 * 1024 * 1024 * 1024] {
        admission(
            Table::Athletes,
            ReadBudget {
                bytes,
                ..OBSERVATION_BUDGET
            },
        )?;
    }
    for budget in [
        ReadBudget {
            bytes: 16 * 1024 * 1024 * 1024 + 1,
            ..OBSERVATION_BUDGET
        },
        ReadBudget {
            rows: census_store::MAX_ROWS_PER_TABLE + 1,
            ..OBSERVATION_BUDGET
        },
        ReadBudget {
            record_bytes: 1024 * 1024 + 1,
            ..OBSERVATION_BUDGET
        },
    ] {
        check!(matches!(
            admission(Table::Athletes, budget),
            Err(StoreError::Refused { .. })
        ));
    }
    check!(matches!(
        admission(Table::Schools, OBSERVATION_BUDGET),
        Err(StoreError::Refused { .. })
    ));
    Ok(())
}

#[test]
fn physical_chunk_inventory_fits_the_complete_framed_table_budget() -> TestResult {
    let key = census_store::MAX_ID_BYTES
        .checked_add(64)
        .ok_or("key bound")?;
    let frame = RECORD_BYTES
        .checked_add(key)
        .and_then(|n| n.checked_add(8))
        .ok_or("frame bound")?;
    let sealed_minimum = CHUNK_BYTES
        .checked_sub(frame)
        .and_then(|n| n.checked_add(1))
        .ok_or("sealed minimum")?;
    let headers = OBSERVATION_BUDGET
        .rows
        .checked_mul(8)
        .ok_or("framing bytes")?;
    let framed = OBSERVATION_BUDGET
        .bytes
        .checked_add(headers)
        .ok_or("framed table")?;
    let chunks = framed
        .checked_div(u64::try_from(sealed_minimum)?)
        .and_then(|n| n.checked_add(1))
        .ok_or("chunk bound")?;
    check!(chunks <= u64::try_from(MAX_CHUNKS)?);
    Ok(())
}

#[test]
fn maximum_physical_inventory_roundtrips_within_the_unchanged_metadata_bound() -> TestResult {
    let directory = tempfile::tempdir()?;
    let inventory = inventory(MAX_CHUNKS);
    let reference = object::encode(directory.path(), &inventory)?;
    check!(reference.bytes <= 4 * 1024 * 1024);
    let reopened: Inventory = object::decode(directory.path(), &reference)?;
    check!(eq; reopened.chunks, inventory.chunks);
    check!(eq; reopened.rows, inventory.rows);
    check!(eq; reopened.bytes, inventory.bytes);
    check!(eq; reopened.digest, inventory.digest);
    Ok(())
}

#[test]
fn physical_reader_refuses_excess_chunk_metadata_before_loading_any_chunk() -> TestResult {
    let directory = tempfile::tempdir()?;
    let inventory = inventory(8193);
    let reference = object::encode(directory.path(), &inventory)?;
    let descriptor = descriptor(reference)?;
    let mut callbacks = 0;
    let result = reader::visit(
        directory.path(),
        &descriptor,
        OBSERVATION_BUDGET,
        |_: CanonicalAthlete| {
            callbacks += 1;
            Ok(())
        },
    );
    check!(
        matches!(result, Err(crate::report::ReportError::Store(StoreError::Refused { detail }))
        if detail == "frozen physical observation inventory differs from its descriptor")
    );
    check!(eq; callbacks, 0);
    Ok(())
}

fn inventory(chunks: usize) -> Inventory {
    Inventory {
        version: 1,
        table: Table::Athletes,
        chunks: (0..chunks)
            .map(|_| ObjectRef {
                digest: "a".repeat(64),
                bytes: 4 * 1024 * 1024,
            })
            .collect(),
        rows: 0,
        bytes: 0,
        digest: "b".repeat(64),
    }
}

fn descriptor(
    inventory: ObjectRef,
) -> Result<PhysicalObservationDescriptor, Box<dyn std::error::Error>> {
    Ok(PhysicalObservationDescriptor {
        version: 1,
        store_identity: "c".repeat(64),
        snapshot_sequence: 1,
        input_generation: "d".repeat(64),
        input_digest: "e".repeat(64),
        source_digest: "f".repeat(64),
        profile_id: ProfileId::parse(&"a".repeat(64))?,
        as_of: ProfileDate::parse("2026-10-23")?,
        table: Table::Athletes,
        inventory,
        fingerprint: "b".repeat(64),
        rows: 0,
        bytes: 0,
    })
}
