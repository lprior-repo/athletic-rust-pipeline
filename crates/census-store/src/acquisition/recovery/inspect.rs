use super::*;
use crate::acquisition::codec;
use crate::StoreResult;

impl<'store, 'budget> PreparedRecoverySession<'store, 'budget> {
    pub fn read_backing(mut self) -> StoreResult<PreparedRecoveryRead<'store, 'budget>> {
        if !matches!(self.state, State::Uninspected) {
            return Err(CaptureError::TransitionRefused.into());
        }
        if let Some((terminal, receipt)) = self.take_recorded()? {
            return self.recorded(terminal, receipt);
        }
        self.inspect_unresolved()?;
        Ok(PreparedRecoveryRead::Unresolved(self))
    }
    pub(super) fn take_recorded(
        &mut self,
    ) -> StoreResult<Option<(ImmutableAttemptResult, Receipt)>> {
        let attempt = self
            .facts
            .attempts
            .get_mut(self.index)
            .ok_or(CaptureError::PreparedTargetMismatch)?;
        let Some(terminal) = attempt.terminal.take() else {
            return Ok(None);
        };
        let ReceiptWitness::Present { receipt, raw } =
            std::mem::replace(&mut attempt.receipt, ReceiptWitness::Absent)
        else {
            return Err(CaptureError::CorruptClosure.into());
        };
        drop(raw);
        Ok(Some((terminal, receipt)))
    }
    fn recorded(
        mut self,
        terminal: ImmutableAttemptResult,
        receipt: Receipt,
    ) -> StoreResult<PreparedRecoveryRead<'store, 'budget>> {
        match terminal.terminal() {
            AcquisitionTerminal::PreparedRetentionFailed { .. } => {
                Ok(PreparedRecoveryRead::RetentionFailed(
                    RecordedRetentionFailure { terminal, receipt },
                ))
            }
            AcquisitionTerminal::ResponseRetained { .. } => {
                let occurrence = retained::occurrence(&terminal, &mut self.account)?;
                let result = retained::projection(
                    &self.facts,
                    &self.prepared,
                    self.index,
                    &occurrence,
                    &mut self.account,
                )
                .and_then(|projection| self.consume_recorded().map(|body| (projection, body)));
                match result {
                    Ok((projection, body)) => Ok(PreparedRecoveryRead::Retained(retained::parts(
                        self, body, occurrence, terminal, receipt, projection,
                    ))),
                    Err(consumption_error) => Ok(PreparedRecoveryRead::RetainedUnavailable {
                        occurrence,
                        terminal,
                        receipt,
                        consumption_error,
                    }),
                }
            }
            _ => Err(CaptureError::PreparedTargetMismatch.into()),
        }
    }
    fn consume_recorded(&mut self) -> StoreResult<Vec<u8>> {
        let attempt = self
            .facts
            .attempts
            .get(self.index)
            .ok_or(CaptureError::PreparedTargetMismatch)?;
        codec::reserve_decode(self.prepared.capture.metadata_bytes(), self.account.reads())?;
        crate::acquisition::backing::validate_metadata(
            &self.facts.work,
            attempt,
            &self.prepared.capture,
        )?;
        if self.paths.body_missing(&mut self.account)?.is_some()
            || self.paths.metadata_missing(&mut self.account)?.is_some()
        {
            return Err(CaptureError::RequiredBackingMissing.into());
        }
        let body =
            backing::materialize(&self.paths.body, &self.prepared.capture, &mut self.account)?;
        backing::verify(
            &self.paths.metadata,
            u64::try_from(self.prepared.capture.metadata_bytes().len())
                .map_err(|_| crate::StoreError::CounterOverflow)?,
            self.prepared.capture.metadata_digest(),
            RecoveryPayloadRole::Metadata,
            &mut self.account,
            &mut self.scratch,
        )?;
        Ok(body)
    }
    fn inspect_unresolved(&mut self) -> StoreResult<()> {
        self.state = State::Refused;
        let attempt = self
            .facts
            .attempts
            .get(self.index)
            .ok_or(CaptureError::PreparedTargetMismatch)?;
        let prepared = &self.prepared;
        codec::reserve_decode(prepared.capture.metadata_bytes(), self.account.reads())?;
        crate::acquisition::backing::validate_metadata(
            &self.facts.work,
            attempt,
            &prepared.capture,
        )?;
        let body_missing = self.paths.body_missing(&mut self.account)?;
        let metadata_missing = self.paths.metadata_missing(&mut self.account)?;
        if metadata_missing.is_none() {
            backing::verify(
                &self.paths.metadata,
                u64::try_from(prepared.capture.metadata_bytes().len())
                    .map_err(|_| crate::StoreError::CounterOverflow)?,
                prepared.capture.metadata_digest(),
                RecoveryPayloadRole::Metadata,
                &mut self.account,
                &mut self.scratch,
            )?;
        }
        if let Some(location) = body_missing {
            if prepared.capture.content_bytes() == 0 {
                if !backing::empty_exact(&prepared.capture) {
                    return Err(CaptureError::BackingIntegrity.into());
                }
                self.account.reserve_body(0)?;
                self.state = State::Restoration {
                    body: Vec::new(),
                    loan: RestoreLoan::Ready,
                };
            } else {
                let metadata = match metadata_missing {
                    Some(location) => MissingMetadataV1::Missing { location },
                    None => MissingMetadataV1::Exact,
                };
                self.state = State::Missing(
                    MissingPreparedAssessmentV1::new(location, metadata)
                        .map_err(CaptureError::Domain)?,
                );
            }
        } else {
            let body =
                backing::materialize(&self.paths.body, &prepared.capture, &mut self.account)?;
            self.state = if metadata_missing.is_none() {
                State::Verified(body)
            } else {
                State::Restoration {
                    body,
                    loan: RestoreLoan::Ready,
                }
            };
        }
        Ok(())
    }
    pub fn missing_decision(&mut self) -> StoreResult<MissingPreparedDecisionV1> {
        let State::Missing(assessment) = &self.state else {
            return Err(CaptureError::TransitionRefused.into());
        };
        self.account.assessment_bytes(4096)?;
        self.account.reserve_owned(320)?;
        let target = self
            .target
            .try_clone()
            .map_err(|_| CaptureError::ReadBudgetExceeded {
                resource: "allocation",
            })?;
        Ok(MissingPreparedDecisionV1::new(target, assessment.clone()))
    }
}
