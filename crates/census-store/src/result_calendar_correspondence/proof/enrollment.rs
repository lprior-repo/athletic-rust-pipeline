use super::super::*;
use crate::{Store, StoreSnapshot};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::StoreIdentity;
use census_domain::result_school_correspondence::ResultSchoolPriorCapabilityV1;

pub struct ResultCalendarEnrollmentProofV1 {
    bytes: Vec<u8>,
}

impl ResultCalendarEnrollmentProofV1 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(in super::super) fn from_held(
        proof: &wire::Proof,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        Ok(Self {
            bytes: codec::encode(proof, grant)?,
        })
    }
}

impl StoreSnapshot<'_> {
    pub fn result_calendar_enrollment_proof_v1(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<ResultCalendarEnrollmentProofV1>> {
        guarded(grant, |grant| {
            self.with_roster_source_snapshot_v1(|snapshot, meta, journal, receipts| {
                let format = read::snapshot_format(snapshot, meta, grant)?;
                if format.lifecycle_revision != Some(LIFECYCLE) {
                    [meta, journal, receipts]
                        .into_iter()
                        .try_for_each(|table| read::namespace(snapshot, table, NAME, &[], grant))?;
                    return Ok(None);
                }
                let proof = read::enrollment(snapshot, meta, journal, receipts, grant)?;
                ResultCalendarEnrollmentProofV1::from_held(&proof, grant).map(Some)
            })
        })
    }
}

impl Store {
    pub fn read_result_calendar_enrollment_proof_v1(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ResultCalendarEnrollmentProofV1> {
        let _lock = self.lock_appends();
        self.snapshot_locked()
            .result_calendar_enrollment_proof_v1(grant)?
            .ok_or_else(|| Error::EnrollmentRequired.into())
    }
}

pub fn replay_result_calendar_enrollment_proof_v1(
    bytes: &[u8],
    store: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultSchoolPriorCapabilityV1> {
    guarded(grant, |grant| {
        let proof: wire::Proof = codec::decode(bytes, grant)?;
        verify(&proof, store, grant)
    })
}

pub(in super::super) fn verify(
    proof: &wire::Proof,
    identity: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultSchoolPriorCapabilityV1> {
    if proof.schema != "result-calendar-enrollment-proof-v1" {
        return Err(Error::Corrupt("enrollment proof schema").into());
    }
    let enrollment_bytes = proof.enrollment.get().as_bytes();
    let record: wire::Enrollment = codec::decode(enrollment_bytes, grant)?;
    wire::validate_enrollment_command(&record.command)?;
    let prior_school = record.prior_school.get().as_bytes();
    let receipt: crate::Receipt = codec::decode(proof.receipt.get().as_bytes(), grant)?;
    let approval = wire::enrollment_approval(&record.command, grant)?;
    let school = crate::replay_result_school_enrollment_proof_v1(prior_school, identity, grant)?;
    if record.schema != "result-calendar-enrollment-v1"
        || &record.command.store != identity
        || record.command.prior_result_school_enrollment != codec::digest(prior_school, grant)?
        || record.approval_digest != codec::digest(&approval, grant)?
        || record.committed_at < record.command.reviewed_at
        || record.initial_inventory != wire::initial(&record.command, grant)?
    {
        return Err(Error::Corrupt("enrollment proof binding").into());
    }
    let enrollment_digest = codec::digest(enrollment_bytes, grant)?;
    if receipt.operation != wire::enrollment_operation(&record.command)
        || receipt.digest != enrollment_digest.as_str()
        || receipt.at != record.committed_at.to_rfc3339()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(Error::Corrupt("enrollment receipt binding").into());
    }
    Ok(school)
}

pub fn require_result_calendar_school_predecessor_v1(
    calendar_bytes: &[u8],
    school_bytes: &[u8],
    store: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultSchoolPriorCapabilityV1> {
    guarded(grant, |grant| {
        let proof: wire::Proof = codec::decode(calendar_bytes, grant)?;
        let capability = verify(&proof, store, grant)?;
        let record: wire::Enrollment = codec::decode(proof.enrollment.get().as_bytes(), grant)?;
        if record.command.prior_result_school_enrollment != codec::digest(school_bytes, grant)? {
            return Err(Error::Corrupt("external school predecessor differs").into());
        }
        let external = crate::replay_result_school_enrollment_proof_v1(school_bytes, store, grant)?;
        if external != capability {
            return Err(Error::Corrupt("school predecessor capability differs").into());
        }
        Ok(external)
    })
}
