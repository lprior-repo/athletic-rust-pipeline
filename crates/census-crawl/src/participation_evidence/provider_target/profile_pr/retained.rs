use super::super::SourceTargetPacketV1;
use super::{bound, invalid, CheckedProfilePrTableV1, MAX_PACKET};
use crate::participation_evidence::recorded::with_grant;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1};

pub struct RetainedProfilePrTableV1 {
    packet: SourceTargetPacketV1,
    root: RetainedCaptureRootV1,
}

impl RetainedProfilePrTableV1 {
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
    pub fn read(&self) -> CrawlResult<CheckedProfilePrTableV1<'_>> {
        Ok(self.packet.read_profile_prs()?)
    }
}

pub fn read_retained(
    held: &RetainedCaptureRootReadV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RetainedProfilePrTableV1> {
    with_grant(grant, |_grant| {
        if held.root().schema != RetainedCaptureRootSchemaV1::ProviderTargetPacketV1 {
            return Err(invalid("profile PR retained root role differs").into());
        }
        bound(
            held.payload().get().len(),
            MAX_PACKET,
            "profile PR retained packet bytes",
        )?;
        let packet: SourceTargetPacketV1 = serde_json::from_str(held.payload().get())
            .map_err(crate::participation_evidence::json)?;
        if packet.reference()?.as_str() != held.root().key {
            return Err(invalid("profile PR retained packet key differs").into());
        }
        packet.read_profile_prs()?;
        Ok(RetainedProfilePrTableV1 {
            packet,
            root: held.root().clone(),
        })
    })
}
