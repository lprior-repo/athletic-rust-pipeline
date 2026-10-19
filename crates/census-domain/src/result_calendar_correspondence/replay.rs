use super::*;
use crate::model::StoreIdentity;
use std::borrow::Cow;

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

pub struct ResultCalendarProjectionV1<'a> {
    records: &'a [ResultCalendarReviewedRecordV1],
    states: Vec<State>,
    inventory: ResultCalendarInventoryV1,
}

impl<'a> ResultCalendarProjectionV1<'a> {
    pub fn records(&self) -> &'a [ResultCalendarReviewedRecordV1] {
        self.records
    }
    pub fn inventory(&self) -> &ResultCalendarInventoryV1 {
        &self.inventory
    }
    pub fn active_records(&self) -> impl Iterator<Item = &'a ResultCalendarReviewedRecordV1> + '_ {
        self.records
            .iter()
            .zip(&self.states)
            .filter_map(|(record, state)| matches!(state, State::Active).then_some(record))
    }
    pub fn active_record_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.states
            .iter()
            .enumerate()
            .filter_map(|(index, state)| matches!(state, State::Active).then_some(index))
    }
}

pub fn replay_result_calendar_records_v1<'a>(
    store: &StoreIdentity,
    initial: &ResultCalendarInventoryV1,
    records: &'a [ResultCalendarReviewedRecordV1],
    receipts: &[ResultCalendarReceiptBindingV1],
    expected: &ResultCalendarInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarProjectionV1<'a>> {
    let result = replay(store, initial, records, receipts, expected, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn replay<'a>(
    store: &StoreIdentity,
    initial: &ResultCalendarInventoryV1,
    records: &'a [ResultCalendarReviewedRecordV1],
    receipts: &[ResultCalendarReceiptBindingV1],
    expected: &ResultCalendarInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarProjectionV1<'a>> {
    grant.ensure_available()?;
    if records.len() > MAX_RESULT_CALENDAR_RECORDS
        || records.len() != receipts.len()
        || initial.version != 1
        || initial.records != 0
    {
        return Err(ResultCalendarCorrespondenceError::Capacity);
    }
    grant.charge_bytes(
        records
            .len()
            .checked_mul(std::mem::size_of::<State>())
            .ok_or(ResultCalendarCorrespondenceError::Capacity)?,
    )?;
    let mut states = Vec::new();
    states
        .try_reserve_exact(records.len())
        .map_err(|_| ResultCalendarCorrespondenceError::Capacity)?;
    let mut inventory = Cow::Borrowed(initial);
    for (index, (record, receipt)) in records.iter().zip(receipts).enumerate() {
        validate::record(record, grant)?;
        if &record.command.store != store
            || record.command.expected_inventory != *inventory
            || record.command.operation != receipt.operation
            || record.record_digest != receipt.record_digest
        {
            return Err(ResultCalendarCorrespondenceError::InventoryMismatch);
        }
        let prefix = records
            .get(..index)
            .ok_or(ResultCalendarCorrespondenceError::Capacity)?;
        let state = transition(record, prefix, &mut states, grant)?;
        states.push(state);
        inventory = Cow::Owned(next_result_calendar_inventory_v1(
            &inventory, record, grant,
        )?);
    }
    if inventory.as_ref() != expected {
        return Err(ResultCalendarCorrespondenceError::InventoryMismatch);
    }
    Ok(ResultCalendarProjectionV1 {
        records,
        states,
        inventory: inventory.into_owned(),
    })
}

fn transition(
    record: &ResultCalendarReviewedRecordV1,
    prefix: &[ResultCalendarReviewedRecordV1],
    states: &mut [State],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<State> {
    let mut latest = None;
    for (index, previous) in prefix.iter().enumerate() {
        grant.charge_steps(1)?;
        if previous.command.operation == record.command.operation {
            return Err(ResultCalendarCorrespondenceError::PredecessorConflict);
        }
        if same_chain(&previous.command.source, &record.command.source) {
            latest = Some(index);
        }
    }
    match &record.command.action {
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        } if latest.is_none() => Ok(State::Active),
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::After(prior),
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
        ResultCalendarActionV1::Revoke { prior } => {
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
        _ => Err(ResultCalendarCorrespondenceError::PredecessorConflict),
    }
}

fn predecessor(
    prior: &ResultCalendarRecordReferenceV1,
    latest: Option<usize>,
    prefix: &[ResultCalendarReviewedRecordV1],
    states: &mut [State],
    source: &ResultCalendarSourceClaimV1,
    transition: Transition,
) -> Result<()> {
    let index = latest.ok_or(ResultCalendarCorrespondenceError::PredecessorConflict)?;
    let previous = prefix
        .get(index)
        .ok_or(ResultCalendarCorrespondenceError::PredecessorConflict)?;
    let state = states
        .get_mut(index)
        .ok_or(ResultCalendarCorrespondenceError::PredecessorConflict)?;
    if prior.operation != previous.command.operation || prior.digest != previous.record_digest {
        return Err(ResultCalendarCorrespondenceError::PredecessorConflict);
    }
    if same_chain(&previous.command.source, source)
        && (matches!(transition, Transition::Accept) || previous.command.source == *source)
    {
        if matches!(transition, Transition::Revoke) && !matches!(state, State::Active) {
            return Err(ResultCalendarCorrespondenceError::PredecessorConflict);
        }
        *state = State::Superseded;
        return Ok(());
    }
    Err(ResultCalendarCorrespondenceError::PredecessorConflict)
}

fn same_chain(left: &ResultCalendarSourceClaimV1, right: &ResultCalendarSourceClaimV1) -> bool {
    left.result == right.result
        && left.person == right.person
        && left.day == right.day
        && left.sport == right.sport
        && left.category == right.category
        && left.year == right.year
}
