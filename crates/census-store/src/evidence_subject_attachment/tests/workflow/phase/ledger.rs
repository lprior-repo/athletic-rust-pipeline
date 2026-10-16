use super::meter::{depth, Meter};
use super::model::Record;
use super::{domain, model};
use crate::public_identity_tests::support::TestResult;

pub(super) fn load(records: &[&Record], meter: &mut Meter) -> TestResult {
    physical(records, meter)?;
    let rows = records.len();
    let legacy = rows
        .checked_add(1)
        .and_then(|n| n.checked_mul(17))
        .ok_or("legacy cost overflow")?;
    for (index, record) in records.iter().enumerate() {
        meter.charge("legacy complete-phase reread", legacy)?;
        meter.singles("entry parse and canonical equality", 2)?;
        meter.charge("domain record decode", 1)?;
        domain::record(record, meter)?;
        meter.charge("fresh decoded record proof issuance", 1)?;
        meter.charge("entry timestamp comparison", 1)?;
        model::closure(
            records.get(..index).ok_or("ledger prefix absent")?,
            record,
            meter,
        )?;
        meter.singles("fresh decoded record and collection origin", 2)?;
    }
    receipts(rows, meter)?;
    domain::inventory(rows, meter)?;
    validation(records, meter)
}

pub(super) fn refresh(records: &[&Record], meter: &mut Meter) -> TestResult {
    physical(records, meter)?;
    meter.charge("retained physical fingerprint equality", 1)?;
    meter.charge("retained positional lengths", 1)?;
    for (index, record) in records.iter().enumerate() {
        meter.charge("retained record traversal", 1)?;
        model::closure(
            records.get(..index).ok_or("refresh prefix absent")?,
            record,
            meter,
        )?;
    }
    receipts(records.len(), meter)?;
    meter.charge("retained receipt lengths", 1)?;
    meter.singles("retained exact receipt equality", records.len())?;
    domain::inventory(records.len(), meter)?;
    meter.charge("retained inventory equality", 1)?;
    validation(records, meter)
}

fn physical(records: &[&Record], meter: &mut Meter) -> TestResult {
    meter.charge("attachment namespace setup", 2)?;
    for _ in records {
        meter.charge("attachment physical inline row", 4)?;
        meter.charge("attachment logical root callback", 1)?;
    }
    Ok(())
}

fn receipts(rows: usize, meter: &mut Meter) -> TestResult {
    meter.singles("receipt slots", rows)?;
    meter.charge("receipt namespace setup", 2)?;
    for index in 0..rows {
        meter.singles("receipt physical row and parse", 2)?;
        meter.singles(
            "receipt operation comparisons",
            index.checked_add(1).ok_or("receipt count overflow")?,
        )?;
    }
    meter.singles("receipt completion", rows)
}

fn validation(records: &[&Record], meter: &mut Meter) -> TestResult {
    domain::projection(records, meter)?;
    for record in records {
        fences(records, record, meter)?;
        history(records, record, meter)?;
    }
    Ok(())
}

pub(super) fn fences(records: &[&Record], record: &Record, meter: &mut Meter) -> TestResult {
    let [capture, token] = &record.inputs;
    for reference in [capture, token] {
        meter.singles(
            "entry input-role declaration",
            depth(record.references(), &reference.digest)?,
        )?;
    }
    check!(model::find(records, &capture.digest, meter)?);
    meter.inline()?;
    meter.charge("capture binding decode group", 3)?;
    check!(model::find(records, &token.digest, meter)?);
    meter.inline()?;
    meter.charge("full retained input-token checks", 5)
}

pub(super) fn history(records: &[&Record], record: &Record, meter: &mut Meter) -> TestResult {
    let [group, witness, capture, token] = &record.historical;
    check!(model::find(records, &witness.digest, meter)?);
    meter.inline()?;
    meter.singles("historical witness parse and canonical equality", 2)?;
    meter.singles("historical witness first origin", record.origin)?;
    for reference in [group, capture, token] {
        meter.singles(
            "historical dependency declaration",
            depth(record.references(), &reference.digest)?,
        )?;
    }
    check!(model::find(records, &group.digest, meter)?);
    meter.inline()?;
    meter.singles("historical group parse canonical equality and one claim", 3)?;
    check!(model::find(records, &capture.digest, meter)?);
    meter.inline()?;
    meter.charge("historical capture binding decode group", 3)?;
    check!(model::find(records, &token.digest, meter)?);
    meter.inline()?;
    meter.charge("historical full retained input-token checks", 5)
}
