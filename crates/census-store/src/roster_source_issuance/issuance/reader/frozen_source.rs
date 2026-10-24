use crate::read::{
    admit_namespace_budget, retained_source_sealed, JournalNamespace, JournalNamespaceContents,
    NamespaceScan, ReadBudget, ReadCounts, RetainedJournalBytes, RetainedJournalReadSource,
};
use crate::{
    RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1, StoreError,
    StoreResult,
};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayContinuationV1, AttachmentReplayGrantV1,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;
mod validate;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CapturedJournalRow {
    pub(super) key: Vec<u8>,
    pub(super) value: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CapturedExternalPrefix {
    pub(super) root: RetainedCaptureRootV1,
    pub(super) rows: Vec<CapturedJournalRow>,
}

pub(super) struct FrozenRosterJournalSource<'a> {
    prefixes: &'a [CapturedExternalPrefix],
    continuation: AttachmentReplayContinuationV1,
}

impl<'a> FrozenRosterJournalSource<'a> {
    pub(super) fn new(
        prefixes: &'a [CapturedExternalPrefix],
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        let result = validate::prefixes(prefixes, grant);
        if result.is_err() {
            grant.poison();
        }
        result?;
        Ok(Self {
            prefixes,
            continuation: grant.checkpoint().map_err(domain)?,
        })
    }

    pub(super) fn require_original_grant(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        self.continuation.require(grant).map_err(domain)
    }

    pub(super) fn read_root(
        &self,
        schema: RetainedCaptureRootSchemaV1,
        key: &str,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<RetainedCaptureRootReadV1>> {
        self.require_original_grant(grant)?;
        let result = crate::evidence_subject_attachment::read_roster_retained_capture_root_v1(
            self, schema, key, grant,
        )
        .and_then(|held| {
            let expected = &self.group(schema.phase(), key)?.root;
            if held.as_ref().is_some_and(|value| value.root() != expected) {
                return Err(outside());
            }
            Ok(held)
        });
        match result {
            Ok(held) => {
                self.require_original_grant(grant)?;
                Ok(held)
            }
            Err(error) => {
                grant.poison();
                Err(error)
            }
        }
    }

    fn group(&self, phase: &str, logical: &str) -> StoreResult<&CapturedExternalPrefix> {
        let root = logical.get(..64).ok_or_else(outside)?;
        self.prefixes
            .binary_search_by(|group| {
                (group.root.schema.phase(), group.root.key.as_str()).cmp(&(phase, root))
            })
            .ok()
            .and_then(|index| self.prefixes.get(index))
            .ok_or_else(outside)
    }

    fn lookup(&self, key: &[u8]) -> StoreResult<Option<&CapturedJournalRow>> {
        let split = key.iter().position(|byte| *byte == 0).ok_or_else(outside)?;
        let phase =
            std::str::from_utf8(key.get(..split).ok_or_else(outside)?).map_err(|_| outside())?;
        let tail = key
            .get(split.checked_add(1).ok_or_else(outside)?..)
            .ok_or_else(outside)?;
        let root =
            std::str::from_utf8(tail.get(..64).ok_or_else(outside)?).map_err(|_| outside())?;
        let group = self.group(phase, root)?;
        Ok(group
            .rows
            .binary_search_by(|row| row.key.as_slice().cmp(key))
            .ok()
            .and_then(|index| group.rows.get(index)))
    }
}

impl retained_source_sealed::Sealed for FrozenRosterJournalSource<'_> {}
impl RetainedJournalReadSource for FrozenRosterJournalSource<'_> {
    fn journal_size(&self, key: &[u8]) -> StoreResult<Option<u32>> {
        self.lookup(key)?
            .map(|row| u32::try_from(row.value.len()).map_err(|_| StoreError::CounterOverflow))
            .transpose()
    }
    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<RetainedJournalBytes<'_>>> {
        Ok(self
            .lookup(key)?
            .map(|row| RetainedJournalBytes::Frozen(&row.value)))
    }
    fn scan_journal_namespace<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        mut admit: impl FnMut(&mut G) -> StoreResult<()>,
        mut visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        admit_namespace_budget(budget)?;
        let group = self.group(namespace.phase(), namespace.logical_prefix())?;
        let prefix = namespace.physical_prefix()?;
        let mut scan = NamespaceScan::new(namespace, budget)?;
        let start = group
            .rows
            .partition_point(|row| row.key.as_slice() < prefix.as_slice());
        for row in group
            .rows
            .get(start..)
            .ok_or_else(outside)?
            .iter()
            .take_while(|row| row.key.starts_with(&prefix))
        {
            admit(state)?;
            scan.charge_visit()?;
            scan.accept(&row.key, &row.value)?;
            visit(state, &row.key, &row.value)?;
        }
        Ok(scan.finish())
    }
    fn visit_receipt_prefix<G>(
        &self,
        _prefix: &[u8],
        _budget: ReadBudget,
        _state: &mut G,
        _admit: impl FnMut(&mut G) -> StoreResult<()>,
        _visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        Err(outside())
    }
}

fn outside() -> StoreError {
    crate::roster_source_issuance::RosterSourceIssuanceStoreError::Corrupt(
        "frozen roster request outside complete prefix closure",
    )
    .into()
}

fn domain(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> StoreError {
    crate::EvidenceSubjectAttachmentStoreError::Domain(error).into()
}
