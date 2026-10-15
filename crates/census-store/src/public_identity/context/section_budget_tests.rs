use super::storage::RETAINED_SECTION_BYTES;
use super::test_fixture::{full_inputs, read_inputs, records, stage_records};
use super::{wire, Inputs};
use crate::public_identity_tests::support::{fixture, retained_state, TestResult};
use crate::{StoreError, StoreResult};
use census_domain::model::{SourceAthleteObservation, SourceNamespace, SourceObservation};

const ROWS: usize = 32;
const ROW_BYTES: usize = 512 * 1024;

fn sized_inputs(extra: usize) -> TestResult<Inputs> {
    let mut inputs = full_inputs()?;
    inputs.observations = (0..ROWS)
        .map(|index| sized_observation(index, if index + 1 == ROWS { extra } else { 0 }))
        .collect::<TestResult<_>>()?;
    Ok(inputs)
}

fn sized_observation(index: usize, extra: usize) -> TestResult<SourceObservation> {
    let mut value = SourceObservation::Athlete(SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        format!("retained-budget-{index}"),
        "",
        "Synthetic bounded athlete",
        "2026-10-14",
    ));
    let length = encoded_length(index, &value)?;
    let SourceObservation::Athlete(row) = &mut value else {
        return Err("expected athlete observation".into());
    };
    row.source_row_key = "x".repeat(
        ROW_BYTES
            .checked_add(extra)
            .and_then(|target| target.checked_sub(length))
            .ok_or("fixture byte arithmetic")?,
    );
    check!(eq; encoded_length(index, &value)?, ROW_BYTES + extra);
    Ok(value)
}

fn encoded_length(index: usize, value: &SourceObservation) -> TestResult<usize> {
    let key = format!("observation:{index:020}");
    let entry = wire::Entry {
        key: &key,
        at: "",
        payload: wire::Payload {
            kind: "observation",
            value,
        },
    };
    serde_json::to_vec(&entry)?
        .len()
        .checked_add(1)
        .ok_or_else(|| "fixture length".into())
}

fn exceeds_section<T>(result: StoreResult<T>) -> TestResult {
    match result {
        Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        }) => {
            check!(eq; what, "retained authority section bytes");
            check!(eq; needed, RETAINED_SECTION_BYTES + 1);
            check!(eq; available, RETAINED_SECTION_BYTES);
        }
        Err(error) => return Err(format!("unexpected section refusal: {error:?}").into()),
        Ok(_) => return Err("oversized retained section accepted".into()),
    }
    Ok(())
}

#[test]
fn retained_section_exact_byte_boundary_roundtrips_with_other_sections() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = sized_inputs(0)?;
    let staged = inputs.stage(&store, writer.store_identity())?;
    let restored = read_inputs(&store, staged.object_id())?;
    check!(eq; restored.observations.len(), ROWS);
    check!(eq; restored.athletes, inputs.athletes);
    check!(eq; restored.cases, inputs.cases);
    check!(eq; restored.verdicts, inputs.verdicts);
    let bytes = restored.observations.iter().enumerate().try_fold(
        0usize,
        |sum, (index, row)| -> TestResult<usize> {
            sum.checked_add(encoded_length(index, row)?)
                .ok_or_else(|| "fixture sum".into())
        },
    )?;
    check!(eq; u64::try_from(bytes)?, RETAINED_SECTION_BYTES);
    Ok(())
}

#[test]
fn retained_section_one_byte_over_refuses_during_measure_before_any_writes() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = sized_inputs(1)?;
    let before = retained_state(&store)?;
    exceeds_section(inputs.stage(&store, writer.store_identity()))?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn resealed_context_one_byte_over_section_limit_refuses_without_writes() -> TestResult {
    replay_refuses_oversized_section(false)
}

#[test]
fn replay_charges_section_bytes_before_typed_observation_deserialization() -> TestResult {
    replay_refuses_oversized_section(true)
}

fn replay_refuses_oversized_section(invalid_type: bool) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let staged = sized_inputs(0)?.stage(&store, writer.store_identity())?;
    let mut values = records(&store, staged.object_id())?;
    let last = values
        .iter_mut()
        .rev()
        .find(|value| {
            value
                .pointer("/payload/kind")
                .and_then(serde_json::Value::as_str)
                == Some("observation")
        })
        .ok_or("last observation")?;
    if invalid_type {
        *last
            .pointer_mut("/payload/value")
            .ok_or("observation value")? = serde_json::json!("");
        let overhead = serde_json::to_vec(last)?.len() + 1;
        *last
            .pointer_mut("/payload/value")
            .ok_or("observation value")? = serde_json::json!("x".repeat(ROW_BYTES + 1 - overhead));
    } else {
        let field = last
            .pointer_mut("/payload/value/source_row_key")
            .ok_or("source row key")?;
        let text = field.as_str().ok_or("source row text")?;
        *field = serde_json::json!(format!("{text}x"));
    }
    check!(eq; serde_json::to_vec(last)?.len() + 1, ROW_BYTES + 1);
    let changed = stage_records(&store, &values)?;
    check!(ne; changed.object_id(), staged.object_id());
    let before = retained_state(&store)?;
    exceeds_section(read_inputs(&store, changed.object_id()))?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
