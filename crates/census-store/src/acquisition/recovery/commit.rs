use super::*;
use crate::acquisition::{codec, read};
use crate::{Receipt, StoreResult};

impl<'budget> PreparedRecoverySession<'_, 'budget> {
    pub fn finish(mut self) -> StoreResult<CommittedRetainedBody<'budget>> {
        let State::Verified(body) = std::mem::replace(&mut self.state, State::Refused) else {
            return Err(CaptureError::TransitionRefused.into());
        };
        self.require_unresolved()?;
        self.verify_fixed_backing()?;
        self.account.reserve_owned(192)?;
        let attempt = self.target.attempt().try_clone().map_err(allocation)?;
        let prepared_digest = self
            .target
            .prepared_digest()
            .try_clone()
            .map_err(allocation)?;
        let terminal = ImmutableAttemptResult::new(
            CaptureProtocolVersion::V2,
            attempt,
            AcquisitionTerminal::ResponseRetained {
                prepared_digest,
                disposition: self.prepared.disposition,
            },
        )?;
        let occurrence = retained::occurrence(&terminal, &mut self.account)?;
        let projection = retained::projection(
            &self.facts,
            &self.prepared,
            self.index,
            &occurrence,
            &mut self.account,
        )?;
        let receipt = self.new_receipt(&terminal)?;
        let result = self.prepared.disposition != CaptureDispositionV1::RetryableResponse;
        self.write_terminal(
            &terminal,
            &receipt,
            if result { Some(&occurrence) } else { None },
        )?;
        Ok(retained::parts(
            self, body, occurrence, terminal, receipt, projection,
        ))
    }
    pub fn close_missing(
        mut self,
        decision: &MissingPreparedDecisionV1,
    ) -> StoreResult<RecordedRetentionFailure> {
        if let Some((terminal, receipt)) = self.take_recorded()? {
            if matches!(terminal.terminal(), AcquisitionTerminal::PreparedRetentionFailed { decision: held, .. } if held == decision)
            {
                codec::repeat(self.store)?;
                return Ok(RecordedRetentionFailure { terminal, receipt });
            }
            return Err(CaptureError::AttemptTerminalConflict.into());
        }
        self.require_unresolved()?;
        let State::Missing(assessment) = std::mem::replace(&mut self.state, State::Refused) else {
            return Err(CaptureError::TransitionRefused.into());
        };
        self.account.assessment_bytes(4096)?;
        self.account.reserve_owned(320)?;
        let owned = MissingPreparedDecisionV1::new(
            self.target.try_clone().map_err(allocation)?,
            assessment.clone(),
        );
        if &owned != decision {
            return Err(CaptureError::PreparedTargetMismatch.into());
        }
        self.recheck_missing(&assessment)?;
        self.account.reserve_owned(192)?;
        let terminal = ImmutableAttemptResult::new(
            CaptureProtocolVersion::V2,
            self.target.attempt().try_clone().map_err(allocation)?,
            AcquisitionTerminal::PreparedRetentionFailed {
                prepared_digest: self
                    .target
                    .prepared_digest()
                    .try_clone()
                    .map_err(allocation)?,
                decision: owned,
            },
        )?;
        let receipt = self.new_receipt(&terminal)?;
        self.write_terminal(&terminal, &receipt, None)?;
        Ok(RecordedRetentionFailure { terminal, receipt })
    }
    fn require_unresolved(&self) -> StoreResult<()> {
        let attempt = self
            .facts
            .attempts
            .get(self.index)
            .ok_or(CaptureError::PreparedTargetMismatch)?;
        if attempt.terminal.is_some()
            || !matches!(attempt.receipt, ReceiptWitness::Absent)
            || self.facts.result.is_some()
            || self.index.checked_add(1) != Some(self.facts.attempts.len())
        {
            return Err(CaptureError::PreparedTargetMismatch.into());
        }
        Ok(())
    }
    fn new_receipt(&mut self, terminal: &ImmutableAttemptResult) -> StoreResult<Receipt> {
        self.account.reserve_owned(4096)?;
        let operation = read::terminal_operation(terminal.attempt(), CaptureProtocolVersion::V2);
        let digest = digest(terminal)?;
        let at = crate::clock::Clock::today(&crate::clock::SystemClock);
        Ok(Receipt {
            operation,
            digest: retained::text(digest.as_str(), &mut self.account)?,
            at,
            appended: 0,
            credited: false,
        })
    }
    fn write_terminal(
        &mut self,
        terminal: &ImmutableAttemptResult,
        receipt: &Receipt,
        result: Option<&AcquisitionOccurrenceRefV1>,
    ) -> StoreResult<()> {
        self.account
            .reserve_owned(3 * MAX_CAPTURE_RECORD_BYTES + 8192)?;
        let receipt_bytes = crate::records::codec::encode_canonical(receipt)?;
        let mut batch = self.store.db.batch();
        self.account.staged_key_probe()?;
        codec::stage(
            self.store,
            &mut batch,
            &format!("{}/terminal", read::attempt_prefix(terminal.attempt())),
            terminal,
            CaptureProtocolVersion::V2,
        )?;
        if let Some(occurrence) = result {
            self.account.reserve_owned(192)?;
            let occurrence = occurrence.try_clone().map_err(allocation)?;
            self.account.staged_key_probe()?;
            codec::stage(
                self.store,
                &mut batch,
                &format!("work/{}/result", terminal.attempt().work_id()),
                &WorkResult {
                    version: CaptureProtocolVersion::V2,
                    occurrence,
                },
                CaptureProtocolVersion::V2,
            )?;
        }
        batch.insert(
            &self.store.receipts,
            receipt.operation.as_bytes(),
            receipt_bytes,
        );
        codec::commit(batch)
    }
    fn recheck_missing(&mut self, assessment: &MissingPreparedAssessmentV1) -> StoreResult<()> {
        let body = self.paths.body_missing(&mut self.account)?;
        let metadata = self.paths.metadata_missing(&mut self.account)?;
        if body.is_none() {
            backing::verify(
                &self.paths.body,
                self.prepared.capture.content_bytes(),
                self.prepared.capture.content_digest(),
                RecoveryPayloadRole::Body,
                &mut self.account,
                &mut self.scratch,
            )?;
        }
        if metadata.is_none() {
            backing::verify(
                &self.paths.metadata,
                u64::try_from(self.prepared.capture.metadata_bytes().len())
                    .map_err(|_| crate::StoreError::CounterOverflow)?,
                self.prepared.capture.metadata_digest(),
                RecoveryPayloadRole::Metadata,
                &mut self.account,
                &mut self.scratch,
            )?;
        }
        if body.is_none() {
            return Err(CaptureError::ExactRecoveryAvailable.into());
        }
        let observed_metadata = match metadata {
            Some(location) => MissingMetadataV1::Missing { location },
            None => MissingMetadataV1::Exact,
        };
        if body != Some(assessment.body()) || observed_metadata != assessment.metadata() {
            return Err(CaptureError::PreparedAssessmentChanged.into());
        }
        Ok(())
    }
}
fn allocation(_: std::collections::TryReserveError) -> crate::StoreError {
    CaptureError::ReadBudgetExceeded {
        resource: "allocation",
    }
    .into()
}
