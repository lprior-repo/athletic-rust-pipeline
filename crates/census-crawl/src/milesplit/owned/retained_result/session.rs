use super::{
    acquisition_time, check_locator, interpretation, RetainedResultError,
    VerifiedRetainedOwnedResult,
};
use crate::milesplit::{
    parse_owned_meet, read_retained_acquisition_from, CaptureReadBudget, OwnedAcquisitionRef,
    OwnedPerformance, OwnedRejectionKind, RetainedJournalView, VerifiedOwnedAcquisition,
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(in crate::milesplit) const MAX_SESSION_CAPTURES: usize = 16;

#[derive(Debug)]
pub(super) struct ParsedCapture {
    pub(super) acquisition: VerifiedOwnedAcquisition,
    pub(super) acquired_at: DateTime<Utc>,
    pub(super) rows: Vec<OwnedPerformance>,
    positions: Vec<(usize, usize)>,
}

pub(in crate::milesplit) struct OwnedResultSession<'a, V> {
    view: &'a V,
    budget: CaptureReadBudget,
    captures: BTreeMap<String, Arc<ParsedCapture>>,
}

impl<'a, V: RetainedJournalView> OwnedResultSession<'a, V> {
    pub(in crate::milesplit) fn new(view: &'a V, budget: CaptureReadBudget) -> Self {
        Self {
            view,
            budget,
            captures: BTreeMap::new(),
        }
    }

    pub(in crate::milesplit) fn clear(&mut self) {
        self.captures.clear();
    }

    pub(in crate::milesplit) fn read(
        &mut self,
        reference: &OwnedAcquisitionRef,
        locator: &str,
    ) -> Result<VerifiedRetainedOwnedResult, RetainedResultError> {
        let position = check_locator(locator)?;
        let parsed = self.capture(reference)?;
        let row = parsed.row(position)?.clone();
        let interpretation_key = interpretation::verify(self.view, &parsed.acquisition, &row)?;
        Ok(VerifiedRetainedOwnedResult::new(
            self.view.sequence(),
            parsed,
            row,
            interpretation_key,
        ))
    }

    fn capture(
        &mut self,
        reference: &OwnedAcquisitionRef,
    ) -> Result<Arc<ParsedCapture>, RetainedResultError> {
        if let Some(capture) = self.captures.get(reference.manifest_key()) {
            return Ok(Arc::clone(capture));
        }
        if self.captures.len() >= MAX_SESSION_CAPTURES {
            return Err(RetainedResultError::SessionBudget);
        }
        let capture = Arc::new(ParsedCapture::read(self.view, reference, self.budget)?);
        self.captures
            .insert(reference.manifest_key().into(), Arc::clone(&capture));
        Ok(capture)
    }
}

impl ParsedCapture {
    fn read(
        view: &impl RetainedJournalView,
        reference: &OwnedAcquisitionRef,
        budget: CaptureReadBudget,
    ) -> Result<Self, RetainedResultError> {
        let acquisition = read_retained_acquisition_from(view, reference, budget)?;
        let acquired_at = acquisition_time(&acquisition)?;
        let page = parse_owned_meet(acquisition.body(), reference.meet_id())?;
        if page
            .rejected
            .iter()
            .any(|row| row.kind == OwnedRejectionKind::DuplicateResult)
        {
            return Err(RetainedResultError::DuplicateResult);
        }
        let mut positions = Vec::new();
        positions.try_reserve_exact(page.rows.len())?;
        page.rows
            .iter()
            .enumerate()
            .try_for_each(|(row_index, row)| {
                positions.push((check_locator(&row.locator)?, row_index));
                Ok::<(), RetainedResultError>(())
            })?;
        positions.sort_unstable();
        Ok(Self {
            acquisition,
            acquired_at,
            rows: page.rows,
            positions,
        })
    }

    fn row(&self, locator: usize) -> Result<&OwnedPerformance, RetainedResultError> {
        let position = self
            .positions
            .binary_search_by_key(&locator, |(locator, _)| *locator)
            .map_err(|_| RetainedResultError::LocatorMissing)?;
        self.positions
            .get(position)
            .and_then(|(_, row)| self.rows.get(*row))
            .ok_or(RetainedResultError::LocatorMissing)
    }
}
