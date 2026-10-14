use super::capability::*;
use super::{codec, facts::*, read, AcquisitionReadBudget, CaptureError};
use crate::{Store, StoreResult};
use census_domain::model::*;
mod work;

impl Store {
    pub fn arm_acquisition_dispatch(
        &self,
        attempt: ReservedAttempt,
        request: DispatchRequestV1,
    ) -> StoreResult<ArmedDispatch> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = current(self, &attempt.work, &snapshot, &mut budget)?;
        let last = active(&facts, &attempt.attempt)?;
        if !last.dispatches.is_empty()
            || request.target_url() != attempt.work.work.request().original_url()
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        verify_conditional(self, &facts.work, &request, &snapshot, &mut budget)?;
        let dispatch = AcquisitionDispatchId::new(
            attempt.attempt,
            HopOrdinal::new(0).map_err(CaptureError::Domain)?,
        );
        arm(self, attempt.work, dispatch, request)
    }

    pub fn record_acquisition_response(
        &self,
        dispatch: DispatchObservation,
        response: ResponseObservationV1,
    ) -> StoreResult<ObservedResponse> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = current(self, &dispatch.work, &snapshot, &mut budget)?;
        let last = active(&facts, dispatch.armed.dispatch.attempt())?;
        if !last
            .dispatches
            .last()
            .is_some_and(|(armed, observed)| armed == &dispatch.armed && observed.is_none())
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        if !response.is_for_dispatch(&dispatch.armed.request) {
            return Err(CaptureError::TransitionRefused.into());
        }
        let observed = Observed {
            version: VersionOne,
            dispatch: dispatch.armed.dispatch,
            response,
        };
        let mut batch = self.db.batch();
        codec::stage(
            self,
            &mut batch,
            &format!("{}/response", read::dispatch_prefix(&observed.dispatch)),
            &observed,
        )?;
        codec::commit(batch)?;
        Ok(ObservedResponse {
            work: dispatch.work,
            observed,
        })
    }

    pub fn arm_acquisition_redirect(
        &self,
        response: ObservedResponse,
        request: DispatchRequestV1,
    ) -> StoreResult<ArmedDispatch> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = current(self, &response.work, &snapshot, &mut budget)?;
        let last = observed(&facts, &response.observed)?;
        if last
            .dispatches
            .last()
            .is_none_or(|(armed, _)| armed.request.conditional() != request.conditional())
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        if !response.observed.response.redirects_to(&request) {
            return Err(CaptureError::TransitionRefused.into());
        }
        let hop = response
            .observed
            .dispatch
            .hop()
            .next()
            .map_err(CaptureError::Domain)?;
        verify_conditional(self, &facts.work, &request, &snapshot, &mut budget)?;
        let dispatch =
            AcquisitionDispatchId::new(response.observed.dispatch.attempt().clone(), hop);
        arm(self, response.work, dispatch, request)
    }

    pub fn prepare_acquisition_capture(
        &self,
        response: ObservedResponse,
        input: PreparedCaptureInputV1,
        disposition: CaptureDispositionV1,
    ) -> StoreResult<PreparedCapture> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = current(self, &response.work, &snapshot, &mut budget)?;
        let last = observed(&facts, &response.observed)?;
        if response.observed.response.is_redirect()
            || CaptureDispositionV1::for_http_status(response.observed.response.status())
                .map_err(CaptureError::Domain)?
                != disposition
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        super::backing::validate_metadata(&facts.work, last, &input)?;
        let prepared = Prepared {
            version: VersionOne,
            attempt: response.observed.dispatch.attempt().clone(),
            dispatch: response.observed.dispatch,
            capture: input,
            disposition,
        };
        let mut batch = self.db.batch();
        codec::stage(
            self,
            &mut batch,
            &format!("{}/prepared", read::attempt_prefix(&prepared.attempt)),
            &prepared,
        )?;
        codec::commit(batch)?;
        Ok(PreparedCapture {
            work: response.work,
            prepared,
        })
    }
}

fn arm(
    store: &Store,
    work: OwnedAcquisitionWork,
    dispatch: AcquisitionDispatchId,
    request: DispatchRequestV1,
) -> StoreResult<ArmedDispatch> {
    let armed = Armed {
        version: VersionOne,
        dispatch,
        request,
    };
    let mut batch = store.db.batch();
    codec::stage(
        store,
        &mut batch,
        &format!("{}/armed", read::dispatch_prefix(&armed.dispatch)),
        &armed,
    )?;
    codec::commit(batch)?;
    Ok(ArmedDispatch { work, armed })
}
pub(super) fn current(
    store: &Store,
    work: &OwnedAcquisitionWork,
    snapshot: &fjall::Snapshot,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<WorkFacts> {
    work.verify(store)?;
    let facts = read::read_work(snapshot, &store.journal, &store.receipts, &work.id, budget)?;
    if facts.work != work.work {
        return Err(CaptureError::ObligationConflict.into());
    }
    Ok(facts)
}
pub(super) fn active<'a>(
    facts: &'a WorkFacts,
    attempt: &AcquisitionAttemptId,
) -> StoreResult<&'a AttemptFacts> {
    let last = facts
        .attempts
        .last()
        .ok_or(CaptureError::TransitionRefused)?;
    if &last.reservation.attempt != attempt
        || last.terminal.is_some()
        || last.prepared.is_some()
        || facts.result.is_some()
    {
        return Err(CaptureError::TransitionRefused.into());
    }
    Ok(last)
}
pub(super) fn observed<'a>(
    facts: &'a WorkFacts,
    response: &Observed,
) -> StoreResult<&'a AttemptFacts> {
    let last = active(facts, response.dispatch.attempt())?;
    if last
        .dispatches
        .last()
        .and_then(|(_, observed)| observed.as_ref())
        != Some(response)
    {
        return Err(CaptureError::TransitionRefused.into());
    }
    Ok(last)
}
fn verify_conditional(
    store: &Store,
    work: &AcquisitionWorkV1,
    request: &DispatchRequestV1,
    snapshot: &fjall::Snapshot,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    if let ConditionalRequestV1::Pinned { original, .. } = request.conditional() {
        let facts = read::read_work(
            snapshot,
            &store.journal,
            &store.receipts,
            original.attempt().work_id(),
            budget,
        )?;
        if facts.work.request() != work.request()
            || original.attempt().store_identity() != work.slot().store_identity()
        {
            return Err(CaptureError::CacheOccurrenceMismatch.into());
        }
        let retained = read::reusable_body_reference(store.root(), &facts, original, budget)?;
        super::backing::verify_validators(retained.prepared(), request.conditional())?;
    }
    Ok(())
}
