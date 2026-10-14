use super::*;
use crate::StoreResult;

impl PreparedRecoverySession<'_, '_> {
    pub fn prepare_restore(&mut self, source: ExactBodySource<'_>) -> StoreResult<()> {
        match std::mem::replace(&mut self.state, State::Refused) {
            State::Restoration {
                body,
                loan: RestoreLoan::Ready,
            } => {
                self.state = State::Restoration {
                    body,
                    loan: RestoreLoan::Ready,
                };
                Ok(())
            }
            State::Missing(_) => {
                let ExactBodySource::SuppliedLocalCopy(path) = source else {
                    return Err(CaptureError::TransitionRefused.into());
                };
                let prepared = &self.prepared;
                let body = backing::candidate(path, &prepared.capture, &mut self.account)?;
                self.state = State::Restoration {
                    body,
                    loan: RestoreLoan::Ready,
                };
                Ok(())
            }
            state => {
                self.state = state;
                Err(CaptureError::TransitionRefused.into())
            }
        }
    }
    pub fn restoration_io(
        &mut self,
    ) -> StoreResult<(
        &ArchiveWriteFence,
        &PreparedCaptureInputV1,
        &[u8],
        RecoveryArchiveIo<'_>,
    )> {
        let State::Restoration { body, loan, .. } = &mut self.state else {
            return Err(CaptureError::TransitionRefused.into());
        };
        if *loan != RestoreLoan::Ready {
            return Err(CaptureError::TransitionRefused.into());
        }
        *loan = RestoreLoan::Issued;
        let prepared = &self.prepared;
        Ok((
            &self.fence,
            &prepared.capture,
            body,
            RecoveryArchiveIo::new(&mut self.account, &mut self.scratch),
        ))
    }
    pub fn verify_restored(&mut self) -> StoreResult<()> {
        let State::Restoration {
            body,
            loan: RestoreLoan::Issued,
        } = std::mem::replace(&mut self.state, State::Refused)
        else {
            return Err(CaptureError::TransitionRefused.into());
        };
        self.verify_fixed_backing()?;
        self.state = State::Verified(body);
        Ok(())
    }
    pub(super) fn verify_fixed_backing(&mut self) -> StoreResult<()> {
        let prepared = &self.prepared;
        if self.paths.body_missing(&mut self.account)?.is_some()
            || self.paths.metadata_missing(&mut self.account)?.is_some()
        {
            return Err(CaptureError::RequiredBackingMissing.into());
        }
        backing::verify(
            &self.paths.body,
            prepared.capture.content_bytes(),
            prepared.capture.content_digest(),
            RecoveryPayloadRole::Body,
            &mut self.account,
            &mut self.scratch,
        )?;
        backing::verify(
            &self.paths.metadata,
            u64::try_from(prepared.capture.metadata_bytes().len())
                .map_err(|_| crate::StoreError::CounterOverflow)?,
            prepared.capture.metadata_digest(),
            RecoveryPayloadRole::Metadata,
            &mut self.account,
            &mut self.scratch,
        )?;
        Ok(())
    }
}
