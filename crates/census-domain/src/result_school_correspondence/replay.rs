use super::*;
use crate::model::StoreIdentity;

#[derive(Clone, Copy)]
enum State {
    Active,
    Revoked,
    Superseded,
}

enum Transition {
    Accept,
    Revoke,
}

pub struct ResultSchoolProjectionV1<'a> {
    records: &'a [ResultSchoolReviewedRecordV1],
    states: Vec<State>,
    inventory: ResultSchoolInventoryV1,
}

impl<'a> ResultSchoolProjectionV1<'a> {
    pub fn records(&self) -> &'a [ResultSchoolReviewedRecordV1] {
        self.records
    }
    pub fn inventory(&self) -> &ResultSchoolInventoryV1 {
        &self.inventory
    }
    pub fn active_records(&self) -> impl Iterator<Item = &'a ResultSchoolReviewedRecordV1> + '_ {
        self.records
            .iter()
            .zip(&self.states)
            .filter_map(|(record, state)| matches!(state, State::Active).then_some(record))
    }
}

pub fn replay_result_school_records_v1<'a>(
    store: &StoreIdentity,
    initial: &ResultSchoolInventoryV1,
    records: &'a [ResultSchoolReviewedRecordV1],
    receipts: &[ResultSchoolReceiptBindingV1],
    expected: &ResultSchoolInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolProjectionV1<'a>> {
    let result = replay(store, initial, records, receipts, expected, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn replay<'a>(
    store: &StoreIdentity,
    initial: &ResultSchoolInventoryV1,
    records: &'a [ResultSchoolReviewedRecordV1],
    receipts: &[ResultSchoolReceiptBindingV1],
    expected: &ResultSchoolInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolProjectionV1<'a>> {
    if records.len() > MAX_RESULT_SCHOOL_RECORDS
        || records.len() != receipts.len()
        || initial.version != 1
        || initial.records != 0
    {
        return Err(ResultSchoolCorrespondenceError::Capacity);
    }
    grant.charge_bytes(
        records
            .len()
            .checked_mul(std::mem::size_of::<State>())
            .ok_or(ResultSchoolCorrespondenceError::Capacity)?,
    )?;
    let mut states = Vec::new();
    states
        .try_reserve_exact(records.len())
        .map_err(|_| ResultSchoolCorrespondenceError::Capacity)?;
    let mut inventory = initial.clone();
    for (index, (record, receipt)) in records.iter().zip(receipts).enumerate() {
        validate::record(record, grant)?;
        if &record.command.store != store
            || record.command.expected_inventory != inventory
            || record.command.operation != receipt.operation
            || record.record_digest != receipt.record_digest
        {
            return Err(ResultSchoolCorrespondenceError::InventoryMismatch);
        }
        let prefix = records
            .get(..index)
            .ok_or(ResultSchoolCorrespondenceError::Capacity)?;
        let state = transition(record, prefix, &mut states, grant)?;
        states.push(state);
        inventory = next_result_school_inventory_v1(&inventory, record, grant)?;
    }
    if &inventory != expected {
        return Err(ResultSchoolCorrespondenceError::InventoryMismatch);
    }
    Ok(ResultSchoolProjectionV1 {
        records,
        states,
        inventory,
    })
}

fn transition(
    record: &ResultSchoolReviewedRecordV1,
    prefix: &[ResultSchoolReviewedRecordV1],
    states: &mut [State],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<State> {
    let mut latest = None;
    for (index, previous) in prefix.iter().enumerate() {
        grant.charge_steps(1)?;
        if previous.command.operation == record.command.operation {
            return Err(ResultSchoolCorrespondenceError::PredecessorConflict);
        }
        if same_chain(&previous.command.source, &record.command.source) {
            latest = Some(index);
        }
    }
    match &record.command.action {
        ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::First,
        } if latest.is_none() => Ok(State::Active),
        ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::After(prior),
        } => {
            predecessor(
                prior,
                latest,
                prefix,
                states,
                &record.command.source,
                Transition::Accept,
            )?;
            Ok(State::Active)
        }
        ResultSchoolActionV1::Revoke { prior } => {
            predecessor(
                prior,
                latest,
                prefix,
                states,
                &record.command.source,
                Transition::Revoke,
            )?;
            Ok(State::Revoked)
        }
        _ => Err(ResultSchoolCorrespondenceError::PredecessorConflict),
    }
}

fn predecessor(
    prior: &ResultSchoolRecordReferenceV1,
    latest: Option<usize>,
    prefix: &[ResultSchoolReviewedRecordV1],
    states: &mut [State],
    source: &ResultSchoolSourceClaimV1,
    transition: Transition,
) -> Result<()> {
    let index = latest.ok_or(ResultSchoolCorrespondenceError::PredecessorConflict)?;
    let previous = prefix
        .get(index)
        .ok_or(ResultSchoolCorrespondenceError::PredecessorConflict)?;
    let state = states
        .get_mut(index)
        .ok_or(ResultSchoolCorrespondenceError::PredecessorConflict)?;
    if previous.command.source.statement != source.statement
        || previous.command.source.target != source.target
        || prior.operation != previous.command.operation
        || prior.digest != previous.record_digest
        || (matches!(transition, Transition::Revoke) && !matches!(state, State::Active))
    {
        return Err(ResultSchoolCorrespondenceError::PredecessorConflict);
    }
    *state = State::Superseded;
    Ok(())
}

fn same_chain(left: &ResultSchoolSourceClaimV1, right: &ResultSchoolSourceClaimV1) -> bool {
    left.statement.result_root == right.statement.result_root
        && left.statement.row_locator == right.statement.row_locator
        && left.target.school == right.target.school
        && left.target.day == right.target.day
        && left.target.sport == right.target.sport
        && left.target.category == right.target.category
        && left.target.year == right.target.year
}
