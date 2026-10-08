use super::{
    budget, journal_index, RecordedBatch, RecordedEffect, RecordedJournal, RecordingState,
    RecordingUsage, MAX_RECORDED_WORK,
};

pub(super) fn append_state(
    state: &mut RecordingState,
    rows: Vec<RecordedBatch>,
    journal: Vec<RecordedJournal>,
    effect: Option<RecordedEffect>,
) -> crate::CrawlResult<()> {
    let extra = budget::measure(&rows, &journal)?;
    let extra = effect.as_ref().map_or(Ok(extra), |_| {
        extra.admitted(RecordingUsage {
            retained_bytes: budget::EFFECT_STORAGE,
            work: 1,
        })
    })?;
    let usage = state.usage.admitted(extra)?;
    reserve(
        state,
        rows.len(),
        journal.len(),
        usize::from(effect.is_some()),
    )?;
    journal_index::reserve(state, &journal)?;
    state.recorded.rows.extend(rows);
    journal_index::append(state, journal);
    state.usage = usage;
    if let Some(effect) = effect {
        state.recorded.effects.push(effect);
    }
    Ok(())
}

fn reserve(
    state: &mut RecordingState,
    rows: usize,
    journal: usize,
    effects: usize,
) -> crate::CrawlResult<()> {
    state
        .recorded
        .rows
        .try_reserve(rows)
        .map_err(|_| budget::resource("recorded batch allocation", rows, MAX_RECORDED_WORK))?;
    state
        .recorded
        .journal
        .try_reserve(journal)
        .map_err(|_| budget::resource("recorded journal allocation", journal, MAX_RECORDED_WORK))?;
    state
        .recorded
        .effects
        .try_reserve(effects)
        .map_err(|_| budget::resource("recorded effect allocation", effects, MAX_RECORDED_WORK))?;
    Ok(())
}
