use super::super::*;
use super::archive::Prefix;
use crate::read::{
    admit_namespace_budget, retained_source_sealed, JournalNamespace, JournalNamespaceContents,
    NamespaceScan, ReadBudget, ReadCounts, RetainedJournalBytes, RetainedJournalReadSource,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::value::RawValue;

mod combined;
pub(super) use combined::Combined;

pub(super) struct Source<'a> {
    roots: Vec<crate::RetainedCaptureRootV1>,
    rows: Vec<(Vec<u8>, &'a RawValue)>,
}

impl<'a> Source<'a> {
    pub(super) fn new(
        groups: Vec<Prefix<'a>>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        let max = MAX_RESULT_CALENDAR_RECORDS
            .checked_mul(MAX_RESULT_CALENDAR_OBJECTS)
            .ok_or(Error::BudgetExceeded)?;
        if groups.len() > max {
            return Err(Error::BudgetExceeded.into());
        }
        let mut rows = Vec::new();
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(groups.len())
            .map_err(|_| Error::BudgetExceeded)?;
        groups.into_iter().try_for_each(|group| {
            if group.rows.is_empty() || group.rows.len() > 6 {
                return Err(Error::Corrupt("frozen source physical rows").into());
            }
            let prefix = JournalNamespace::new(group.root.schema.phase(), &group.root.key)?
                .physical_prefix()?;
            group.rows.into_iter().try_for_each(|row| {
                grant.charge_steps(1).map_err(grant_error)?;
                if row.key.len()
                    > crate::MAX_JOURNAL_KEY_BYTES
                        .checked_mul(2)
                        .ok_or(Error::BudgetExceeded)?
                {
                    return Err(Error::BudgetExceeded.into());
                }
                grant.charge_bytes(row.key.len()).map_err(grant_error)?;
                let mut key = Vec::new();
                key.try_reserve_exact(row.key.len())
                    .map_err(|_| Error::BudgetExceeded)?;
                STANDARD
                    .decode_vec(&row.key, &mut key)
                    .map_err(|_| Error::Corrupt("frozen source physical key"))?;
                if !key.starts_with(&prefix) {
                    return Err(Error::Corrupt("frozen source prefix ownership").into());
                }
                rows.try_reserve_exact(1)
                    .map_err(|_| Error::BudgetExceeded)?;
                rows.push((key, row.value));
                Ok::<_, StoreError>(())
            })?;
            roots.push(group.root);
            Ok::<_, StoreError>(())
        })?;
        rows.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        if rows.windows(2).any(|pair| match pair {
            [left, right] => left.0 == right.0,
            _ => true,
        }) {
            return Err(Error::Corrupt("duplicate frozen source key").into());
        }
        Ok(Self { roots, rows })
    }

    fn lookup(&self, key: &[u8]) -> Option<&'a RawValue> {
        self.rows
            .binary_search_by(|row| row.0.as_slice().cmp(key))
            .ok()
            .and_then(|index| self.rows.get(index))
            .map(|row| row.1)
    }

    fn owns_namespace(&self, namespace: JournalNamespace<'_>) -> bool {
        self.roots.iter().any(|root| {
            root.schema.phase() == namespace.phase() && root.key == namespace.logical_prefix()
        })
    }

    fn owns_key(&self, key: &[u8]) -> bool {
        self.roots.iter().any(|root| {
            key.strip_prefix(root.schema.phase().as_bytes())
                .and_then(|tail| tail.strip_prefix(&[0]))
                .is_some_and(|tail| tail.starts_with(root.key.as_bytes()))
        })
    }
}

impl retained_source_sealed::Sealed for Source<'_> {}
impl RetainedJournalReadSource for Source<'_> {
    fn journal_size(&self, key: &[u8]) -> StoreResult<Option<u32>> {
        self.lookup(key)
            .map(|raw| u32::try_from(raw.get().len()).map_err(|_| Error::BudgetExceeded.into()))
            .transpose()
    }
    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<RetainedJournalBytes<'_>>> {
        Ok(self
            .lookup(key)
            .map(|raw| RetainedJournalBytes::Frozen(raw.get().as_bytes())))
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
        if !self.owns_namespace(namespace) {
            return Err(Error::SourceMismatch("root outside frozen closure").into());
        }
        let prefix = namespace.physical_prefix()?;
        let start = self
            .rows
            .partition_point(|row| row.0.as_slice() < prefix.as_slice());
        let rows = self
            .rows
            .get(start..)
            .ok_or(Error::Corrupt("frozen source range"))?;
        let mut scan = NamespaceScan::new(namespace, budget)?;
        rows.iter()
            .take_while(|row| row.0.starts_with(&prefix))
            .try_for_each(|(key, raw)| {
                admit(state)?;
                scan.charge_visit()?;
                scan.accept(key, raw.get().as_bytes())?;
                visit(state, key, raw.get().as_bytes())
            })?;
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
        Err(Error::SourceMismatch("receipt namespace outside original capture closure").into())
    }
}
