use super::*;
use crate::{Application, AttachmentContextObjectV1, Store, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentPublicHeadV1;
use census_domain::model::StoreIdentity;

mod apply;
mod inspect;
pub(super) mod ledger;
mod prepare;
mod replay;

pub struct ResultSchoolCorrespondenceInspectionV1<'s> {
    revision: StoreRevision<'s>,
    store: StoreIdentity,
    inventory: ResultSchoolInventoryV1,
    input_token: ResultSchoolDigestV1,
    public_head: AttachmentPublicHeadV1,
    grant: AttachmentReplayGrantV1,
}

pub struct PreparedResultSchoolCorrespondenceV1<'s> {
    revision: StoreRevision<'s>,
    command: ResultSchoolReviewCommandV1,
    approval: Vec<u8>,
    approval_digest: ResultSchoolDigestV1,
    state: PreparedState,
    ledger: ledger::Ledger,
    attachment_fence: crate::read::JournalNamespaceInventory<'s>,
    grant: AttachmentReplayGrantV1,
}

enum PreparedState {
    New(Vec<super::source::Material>),
    Replay(crate::Receipt),
}

impl ResultSchoolCorrespondenceInspectionV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store
    }
    pub fn inventory(&self) -> &ResultSchoolInventoryV1 {
        &self.inventory
    }
    pub fn head(&self) -> &ResultSchoolDigestV1 {
        &self.inventory.digest
    }
    pub fn input_token(&self) -> &ResultSchoolDigestV1 {
        &self.input_token
    }
    pub fn public_head(&self) -> &AttachmentPublicHeadV1 {
        &self.public_head
    }
    pub fn evidence_generation(&self) -> u64 {
        self.revision.evidence_generation()
    }
    pub fn derived_generation(&self) -> u64 {
        self.revision.derived_generation()
    }
}

impl PreparedResultSchoolCorrespondenceV1<'_> {
    pub fn command(&self) -> &ResultSchoolReviewCommandV1 {
        &self.command
    }
    pub fn approval_bytes(&self) -> &[u8] {
        &self.approval
    }
    pub fn approval_digest(&self) -> &ResultSchoolDigestV1 {
        &self.approval_digest
    }
    pub fn is_replay(&self) -> bool {
        matches!(self.state, PreparedState::Replay(_))
    }
}

pub enum ResultSchoolCorrespondencePreparationV1<'s, 'a> {
    New {
        inspection: ResultSchoolCorrespondenceInspectionV1<'s>,
        context: &'a [AttachmentContextObjectV1<'a>],
    },
    ReplayOnly,
}

impl Store {
    pub fn inspect_result_school_correspondence_v1(
        &self,
        grant: AttachmentReplayGrantV1,
    ) -> StoreResult<ResultSchoolCorrespondenceInspectionV1<'_>> {
        let _lock = self.lock_appends();
        let (inspected, grant) = inspect::inspect(self, grant)?;
        Ok(ResultSchoolCorrespondenceInspectionV1 {
            revision: inspected.revision,
            store: inspected.store,
            inventory: inspected.inventory,
            input_token: inspected.input_token,
            public_head: inspected.public_head,
            grant,
        })
    }

    pub fn prepare_result_school_correspondence_v1<'s>(
        &'s self,
        command: ResultSchoolReviewCommandV1,
        preparation: ResultSchoolCorrespondencePreparationV1<'s, '_>,
    ) -> StoreResult<PreparedResultSchoolCorrespondenceV1<'s>> {
        let _lock = self.lock_appends();
        match preparation {
            ResultSchoolCorrespondencePreparationV1::New {
                inspection,
                context,
            } => prepare::prepare(self, command, inspection, context),
            ResultSchoolCorrespondencePreparationV1::ReplayOnly => {
                prepare::prepare_replay(self, command, AttachmentReplayGrantV1::new())
            }
        }
    }

    pub fn apply_result_school_correspondence_v1(
        &self,
        prepared: PreparedResultSchoolCorrespondenceV1<'_>,
        approved: &[u8],
        approval_digest: &ResultSchoolDigestV1,
    ) -> StoreResult<Application> {
        let _lock = self.lock_appends();
        apply::apply(self, prepared, approved, approval_digest)
    }
}
