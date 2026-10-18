use super::super::{
    external, RetainedCaptureLeafV1, RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1,
};
use super::*;
use crate::read::{
    admit_namespace_budget, retained_source_sealed, JournalNamespace, JournalNamespaceContents,
    NamespaceScan, ReadBudget, ReadCounts, ReceiptScan, RetainedJournalBytes,
    RetainedJournalReadSource,
};

mod coverage;
mod scan;

pub struct FrozenRetainedCaptureSourceV1<'a> {
    pub(super) archive: archive::Archive<'a>,
}

impl<'a> FrozenRetainedCaptureSourceV1<'a> {
    pub(crate) fn read(
        bundle: &'a FrozenEvidenceSubjectAttachmentBundleV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        Self::read_bytes(bundle.bytes(), bundle.reference(), grant)
    }

    pub(crate) fn read_bytes(
        bytes: &'a [u8],
        reference: &AttachmentObjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = (|| {
            super::bundle::require_reference(bytes, reference, grant)?;
            Ok(Self {
                archive: archive::read(bytes, grant)?,
            })
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub(crate) fn contains_retained_capture_root(
        &self,
        expected: &super::super::RetainedCaptureRootV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<bool> {
        self.archive
            .prefixes
            .iter()
            .skip(2)
            .try_fold(false, |found, group| {
                steps(grant, 1)?;
                let root = group.root.as_ref().ok_or_else(outside)?;
                if root.schema != expected.schema || root.key != expected.key {
                    return Ok(found);
                }
                if root.payload.digest != expected.payload.digest.as_str()
                    || root.payload.bytes != expected.payload.bytes
                {
                    return Err(Error::Corrupt("frozen external root descriptor differs").into());
                }
                Ok(true)
            })
    }

    pub fn read_retained_capture_root_v1(
        &self,
        schema: RetainedCaptureRootSchemaV1,
        key: &str,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<RetainedCaptureRootReadV1>> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = external::read_owned(self, schema, key, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub fn verify_retained_capture_leaf_v1(
        &self,
        leaf: &RetainedCaptureLeafV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        external::verify_leaf(self, leaf, grant)
    }
}

impl retained_source_sealed::Sealed for FrozenRetainedCaptureSourceV1<'_> {}
impl RetainedJournalReadSource for FrozenRetainedCaptureSourceV1<'_> {
    fn journal_size(&self, key: &[u8]) -> StoreResult<Option<u32>> {
        self.lookup(key)?
            .map(|row| {
                u32::try_from(row.value.get().len()).map_err(|_| StoreError::CounterOverflow)
            })
            .transpose()
    }
    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<RetainedJournalBytes<'_>>> {
        Ok(self
            .lookup(key)?
            .map(|row| RetainedJournalBytes::Frozen(row.value.get().as_bytes())))
    }
    fn scan_journal_namespace<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        scan::journal(self, namespace, budget, state, admit, visit)
    }
    fn visit_receipt_prefix<G>(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        scan::receipts(self, prefix, budget, state, admit, visit)
    }
}

fn outside() -> StoreError {
    Error::Corrupt("frozen request is outside captured closure").into()
}
