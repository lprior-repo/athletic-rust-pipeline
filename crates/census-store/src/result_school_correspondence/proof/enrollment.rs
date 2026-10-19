use super::super::*;
use crate::{Store, StoreSnapshot};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_qualification::PublicIdentityQualificationV3;

pub struct ResultSchoolEnrollmentProofV1 {
    bytes: Vec<u8>,
}
impl ResultSchoolEnrollmentProofV1 {
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
    pub fn result_school_enrollment_proof_v1(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<ResultSchoolEnrollmentProofV1>> {
        guarded(grant, |grant| {
            self.with_roster_source_snapshot_v1(|snapshot, meta, journal, receipts| {
                let format = read::snapshot_format(snapshot, meta, grant)?;
                if !super::super::retains_school_enrollment(format.lifecycle_revision) {
                    return Ok(None);
                }
                let proof = read::enrollment(snapshot, meta, journal, receipts, grant)?;
                ResultSchoolEnrollmentProofV1::from_held(&proof, grant).map(Some)
            })
        })
    }
}

impl Store {
    pub fn read_result_school_enrollment_proof_v1(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ResultSchoolEnrollmentProofV1> {
        let _lock = self.lock_appends();
        self.snapshot_locked()
            .result_school_enrollment_proof_v1(grant)?
            .ok_or_else(|| Error::EnrollmentRequired.into())
    }
}

pub fn replay_result_school_enrollment_proof_v1(
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
    if proof.schema != "result-school-enrollment-proof-v1" {
        return Err(Error::Corrupt("enrollment proof schema").into());
    }
    let record: wire::Enrollment = codec::decode(&proof.enrollment, grant)?;
    wire::validate_enrollment_command(&record.command)?;
    let qualification: PublicIdentityQualificationV3 = codec::decode(&proof.qualification, grant)?;
    let approval = wire::enrollment_approval(&record.command, grant)?;
    if record.schema != "result-school-enrollment-v1"
        || &record.command.store != identity
        || qualification.store_identity() != identity
        || record.command.prior.qualification() != &codec::digest(&proof.qualification, grant)?
        || record.approval_digest != codec::digest(&approval, grant)?
        || record.committed_at < record.command.reviewed_at
        || record.initial_inventory != wire::initial(&record.command, grant)?
    {
        return Err(Error::Corrupt("enrollment proof binding").into());
    }
    prior(&proof.prior, &record.command.prior, identity, grant)?;
    let digest = codec::digest(&proof.enrollment, grant)?;
    let receipt: crate::Receipt = codec::decode(&proof.receipt, grant)?;
    if receipt.operation != wire::enrollment_operation(&record.command)
        || receipt.digest != digest.as_str()
        || receipt.at != record.committed_at.to_rfc3339()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(Error::Corrupt("enrollment receipt binding").into());
    }
    Ok(record.command.prior)
}

fn prior(
    proof: &wire::PriorProof,
    prior: &ResultSchoolPriorCapabilityV1,
    identity: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    match (proof, prior) {
        (wire::PriorProof::IdentityOnly, ResultSchoolPriorCapabilityV1::IdentityOnly { .. }) => {
            Ok(())
        }
        (
            wire::PriorProof::IdentityAndRosterSource {
                enrollment,
                receipt,
            },
            ResultSchoolPriorCapabilityV1::IdentityAndRosterSource {
                roster_enrollment, ..
            },
        ) => {
            if &codec::digest(enrollment, grant)? != roster_enrollment {
                return Err(Error::Corrupt("prior roster enrollment differs").into());
            }
            crate::roster_source_issuance::validate_roster_enrollment_proof_v1(
                enrollment, receipt, identity, grant,
            )
        }
        _ => Err(Error::Corrupt("prior capability proof differs").into()),
    }
}
