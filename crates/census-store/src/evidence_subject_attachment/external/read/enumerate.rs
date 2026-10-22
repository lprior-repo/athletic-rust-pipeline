use super::{owned, Error, RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1};
use crate::read::{JournalNamespace, JournalNamespaceInventory};
use crate::{ReadBudget, StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

impl<'s> StoreSnapshot<'s> {
    pub fn for_each_retained_capture_root_v1(
        &self,
        schema: RetainedCaptureRootSchemaV1,
        budget: ReadBudget,
        grant: &mut AttachmentReplayGrantV1,
        mut visit: impl FnMut(RetainedCaptureRootReadV1) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceInventory<'s>> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = JournalNamespace::whole_phase(schema.phase()).and_then(|namespace| {
            self.for_each_journal_root_with_admission(
                namespace,
                budget,
                grant,
                |grant| {
                    grant
                        .charge_steps(1)
                        .map_err(Error::Domain)
                        .map_err(Into::into)
                },
                |grant, key| {
                    let held = owned(self, schema, key, grant)?
                        .ok_or(Error::Corrupt("missing retained capture root"))?;
                    visit(held)
                },
            )
        });
        if result.is_err() {
            grant.poison();
        }
        result
    }
}
