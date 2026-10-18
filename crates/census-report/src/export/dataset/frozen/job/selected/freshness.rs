use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::QueryProfileV1;
use census_store::StoreSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::export::dataset) struct Freshness {
    attachments: String,
    reviewed_roster: Option<String>,
    school_enrollment: Option<String>,
    school_correspondences: Option<String>,
}

pub(super) fn capture(
    snapshot: &StoreSnapshot<'_>,
    profile: &QueryProfileV1,
) -> ReportResult<Freshness> {
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let attachment_digest = super::super::super::digest(attachments.inventory())?;
    let school_enrollment = snapshot.result_school_enrollment_proof_v1(&mut grant)?;
    let (school_enrollment, school_correspondences) = match school_enrollment {
        Some(proof) => {
            let school =
                snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
            (
                Some(super::super::super::digest(&proof.bytes())?),
                Some(super::super::super::digest(school.inventory())?),
            )
        }
        None => (None, None),
    };
    drop(attachments);
    let reviewed_roster =
        if census_domain::profile_execution::validate_selected_qualification_profile_v4(profile)
            .is_ok()
            || census_domain::profile_execution::validate_selected_qualification_profile_v5(profile)
                .is_ok()
        {
            let mut grant = AttachmentReplayGrantV1::new();
            let roster = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
            Some(super::super::super::digest(
                &roster.authority_head_bytes(&mut grant)?,
            )?)
        } else {
            None
        };
    Ok(Freshness {
        attachments: attachment_digest,
        reviewed_roster,
        school_enrollment,
        school_correspondences,
    })
}

impl Freshness {
    pub(in crate::export::dataset) fn require(
        &self,
        snapshot: &StoreSnapshot<'_>,
        profile: &QueryProfileV1,
    ) -> ReportResult<()> {
        if &capture(snapshot, profile)? != self {
            return Err(super::invalid(
                "selected export job authority changed; retained input is preserved",
            ));
        }
        Ok(())
    }
}
