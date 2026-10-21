use super::RosterSourceEvidenceV1;
use crate::{RetainedCaptureRootV1, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::ReplayLimits;
use serde::Serialize;

#[derive(Serialize)]
struct RetainedAdmission<'a> {
    schema: &'static str,
    store: &'a StoreIdentity,
    inspected_sequence: u64,
    school: &'a census_domain::model::SchoolId,
    program: &'a census_domain::model::TeamId,
    root: &'a RetainedCaptureRootV1,
    admission: &'static str,
    original_http_authorization: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_intake: Option<super::observed::ObservedIntakePins>,
}

#[derive(Serialize)]
struct VerificationGrant<'a> {
    schema: &'static str,
    root: &'a RetainedCaptureRootV1,
    encoded_bytes: usize,
    traversal_steps: usize,
    policy: &'static str,
    original_http_authorization: &'static str,
}

impl RosterSourceEvidenceV1 {
    pub fn retained_source_admission_manifest(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Vec<u8>> {
        self.require_original_grant(grant)?;
        let held = self.retained_root(grant)?;
        let value = RetainedAdmission {
            schema: "roster-retained-source-admission-v1",
            store: self.store_identity(),
            inspected_sequence: self.snapshot_sequence(),
            school: self.requested_school(),
            program: self.requested_program(),
            root: held.root(),
            admission: super::observed::admission_kind(held.root().schema),
            original_http_authorization: super::observed::authorization(held.root().schema),
            observed_intake: self
                .observed
                .as_ref()
                .map(super::observed::pins)
                .transpose()?,
        };
        encode(self, &value, grant)
    }

    pub fn verification_grant_manifest(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Vec<u8>> {
        self.require_original_grant(grant)?;
        let held = self.retained_root(grant)?;
        let value = VerificationGrant {
            schema: "roster-source-verification-grant-policy-v1",
            root: held.root(),
            encoded_bytes: ReplayLimits::SMALL.total_text_bytes,
            traversal_steps: ReplayLimits::SMALL.traversal_steps,
            policy: "one-original-origin-monotonic-usage-poison-on-error-no-reset",
            original_http_authorization: super::observed::authorization(held.root().schema),
        };
        encode(self, &value, grant)
    }
}

fn encode(
    evidence: &RosterSourceEvidenceV1,
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let result = super::super::codec::encode_with_source_grant(value, grant);
    match result {
        Ok(bytes) => {
            evidence.require_original_grant(grant)?;
            Ok(bytes)
        }
        Err(error) => {
            grant.poison();
            Err(error)
        }
    }
}
