use super::{CaptureProvenance, CapturedFetch};
use crate::net::cache::recovery::{publish_recovered_eligible_held, restore_exact_prepared_held};
use crate::net::{FetchError, FetchOutcome, Fetcher};
use census_domain::model::{AcquisitionOccurrenceRefV1, PreparedRecoveryTarget};
use census_store::{
    AcquisitionReadBudget, CaptureOwnerRef, CommittedRetainedBody, ExactBodySource,
    ExistingRecoveryRoute, ImmutableAttemptResult, OwnedAcquisitionWork, PreparedBackingAssessment,
    PreparedRecoveryBudget, PreparedRecoveryRead, Receipt, RetainedCaptureParts, Store,
};

pub use crate::net::cache::recovery::{EligiblePublicationOutcome, EligiblePublicationSkip};

pub enum PreparedRecoveryOutcome {
    Retained {
        captured: Box<CapturedFetch>,
        terminal: ImmutableAttemptResult,
        receipt: Receipt,
        eligible: EligiblePublicationOutcome,
    },
    RetainedUnavailable {
        occurrence: AcquisitionOccurrenceRefV1,
        terminal: ImmutableAttemptResult,
        receipt: Receipt,
        consumption_error: FetchError,
    },
    RetentionFailed {
        terminal: ImmutableAttemptResult,
        receipt: Receipt,
    },
}

impl CapturedFetch {
    pub(crate) fn from_prepared_recovery(
        retained: CommittedRetainedBody<'_>,
    ) -> (Self, ImmutableAttemptResult, Receipt) {
        let RetainedCaptureParts {
            url,
            response_url,
            method,
            status,
            content_digest,
            bytes,
            fetched_at,
            content_type,
            body,
            occurrence,
            original_occurrence,
            prepared,
            terminal,
            receipt,
        } = retained.into_parts();
        let capture = FetchOutcome {
            url,
            response_url,
            method,
            status,
            content_digest,
            bytes,
            fetched_at,
            content_type,
            body,
            from_cache: true,
        };
        let captured = Self {
            capture,
            provenance: CaptureProvenance::retained(occurrence, original_occurrence),
            original_metadata: Some(prepared.into_metadata_bytes()),
        };
        (captured, terminal, receipt)
    }
}

impl Fetcher {
    pub fn recover_prepared_target<'owner>(
        &self,
        store: &Store,
        owner: impl Into<CaptureOwnerRef<'owner>>,
        target: &PreparedRecoveryTarget,
        budget: &mut PreparedRecoveryBudget,
        source: ExactBodySource<'_>,
    ) -> Result<PreparedRecoveryOutcome, FetchError> {
        if !store.matches_http_cache_root(self.cache_dir()) {
            return Err(census_store::CaptureError::CacheRootMismatch.into());
        }
        let read = store
            .begin_prepared_recovery(owner, target, budget)?
            .read_backing()?;
        finish_recovery_read(read, source)
    }

    pub fn recover_prepared_acquisition(
        &self,
        store: &Store,
        work: OwnedAcquisitionWork,
        source: ExactBodySource<'_>,
    ) -> Result<CapturedFetch, FetchError> {
        let mut reads = AcquisitionReadBudget::for_work();
        let route = store.read_existing_recovery_target(work.owner(), work.work(), &mut reads)?;
        let ExistingRecoveryRoute::Prepared(target) = route else {
            return Err(census_store::CaptureError::PreparedTargetMismatch.into());
        };
        let mut budget = PreparedRecoveryBudget::admit(reads.selection_usage(), 1)?;
        if !store.matches_http_cache_root(self.cache_dir()) {
            return Err(census_store::CaptureError::CacheRootMismatch.into());
        }
        let read = store
            .begin_prepared_recovery(work.owner(), &target, &mut budget)?
            .read_backing()?;
        match read {
            PreparedRecoveryRead::Unresolved(session)
                if matches!(
                    session.assessment()?,
                    PreparedBackingAssessment::MissingBody(_)
                ) && matches!(source, ExactBodySource::NoneSupplied) =>
            {
                Err(census_store::CaptureError::PreparedTargetMismatch.into())
            }
            read => match finish_recovery_read(read, source)? {
                PreparedRecoveryOutcome::Retained { captured, .. } => Ok(*captured),
                PreparedRecoveryOutcome::RetainedUnavailable {
                    consumption_error, ..
                } => Err(consumption_error),
                PreparedRecoveryOutcome::RetentionFailed { .. } => {
                    Err(census_store::CaptureError::RetainedOutcomeUnknown.into())
                }
            },
        }
    }
}

fn finish_recovery_read(
    read: PreparedRecoveryRead<'_, '_>,
    source: ExactBodySource<'_>,
) -> Result<PreparedRecoveryOutcome, FetchError> {
    match read {
        PreparedRecoveryRead::Retained(retained) => {
            let capacity = super::super::cache::recovery::archive_capacity()?;
            Ok(retained_outcome(retained, capacity))
        }
        PreparedRecoveryRead::RetainedUnavailable {
            occurrence,
            terminal,
            receipt,
            consumption_error,
        } => Ok(PreparedRecoveryOutcome::RetainedUnavailable {
            occurrence,
            terminal,
            receipt,
            consumption_error: consumption_error.into(),
        }),
        PreparedRecoveryRead::RetentionFailed(failure) => {
            let (terminal, receipt) = failure.into_parts();
            Ok(PreparedRecoveryOutcome::RetentionFailed { terminal, receipt })
        }
        PreparedRecoveryRead::Unresolved(session) => resolve_unresolved(session, source, None),
    }
}

fn admitted_capacity(pre_admitted: Option<u64>) -> Result<u64, FetchError> {
    if let Some(capacity) = pre_admitted {
        return Ok(capacity);
    }
    super::super::cache::recovery::archive_capacity()
}

pub(super) fn resolve_unresolved(
    mut session: census_store::PreparedRecoverySession<'_, '_>,
    source: ExactBodySource<'_>,
    capacity_override: Option<u64>,
) -> Result<PreparedRecoveryOutcome, FetchError> {
    match session.assessment()? {
        PreparedBackingAssessment::Exact => {
            let capacity = admitted_capacity(capacity_override)?;
            return Ok(retained_outcome(session.finish()?, capacity));
        }
        PreparedBackingAssessment::RestorationRequired => {
            session.prepare_restore(ExactBodySource::NoneSupplied)?;
        }
        PreparedBackingAssessment::MissingBody(_) => match source {
            ExactBodySource::NoneSupplied => {
                let decision = session.missing_decision()?;
                let (terminal, receipt) = session.close_missing(&decision)?.into_parts();
                return Ok(PreparedRecoveryOutcome::RetentionFailed { terminal, receipt });
            }
            ExactBodySource::SuppliedLocalCopy(path) => {
                session.prepare_restore(ExactBodySource::SuppliedLocalCopy(path))?;
            }
        },
    }
    {
        let capacity = admitted_capacity(capacity_override)?;
        let (fence, prepared, body, io) = session.restoration_io()?;
        restore_exact_prepared_held(fence, prepared, body, capacity, io)?;
        session.verify_restored()?;
        Ok(retained_outcome(session.finish()?, capacity))
    }
}

fn retained_outcome(
    mut retained: CommittedRetainedBody<'_>,
    capacity: u64,
) -> PreparedRecoveryOutcome {
    if let Err(error) = retained.reserve_consumer_owned(std::mem::size_of::<CapturedFetch>()) {
        let parts = retained.into_parts();
        return PreparedRecoveryOutcome::RetainedUnavailable {
            occurrence: parts.occurrence,
            terminal: parts.terminal,
            receipt: parts.receipt,
            consumption_error: error.into(),
        };
    }
    let eligible = match retained.publication_io() {
        Ok((fence, prepared, occurrence, io)) => {
            publish_recovered_eligible_held(fence, prepared, occurrence, capacity, io)
        }
        Err(error) => {
            EligiblePublicationOutcome::Skipped(EligiblePublicationSkip::NotAdmitted(error))
        }
    };
    let (captured, terminal, receipt) = CapturedFetch::from_prepared_recovery(retained);
    PreparedRecoveryOutcome::Retained {
        captured: Box::new(captured),
        terminal,
        receipt,
        eligible,
    }
}
