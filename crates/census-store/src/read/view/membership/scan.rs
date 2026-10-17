use fjall::Readable;
use sha2::{Digest, Sha256};

use super::{
    JournalNamespace, JournalNamespaceContents, JournalNamespaceFingerprint, ReadBudget,
    StoreSnapshot,
};
use crate::{StoreError, StoreResult, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES};

const MAX_NAMESPACE_ROWS: u64 = 25_000;
const MAX_NAMESPACE_BYTES: u64 = 64 * 1024 * 1024;

impl<'s> StoreSnapshot<'s> {
    pub(super) fn scan_journal_namespace(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        mut visit: impl FnMut(&[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceFingerprint<'s>> {
        self.scan_journal_namespace_with_admission(
            namespace,
            budget,
            &mut (),
            |_| Ok(()),
            |_, key, raw| visit(key, raw),
        )
    }

    pub(super) fn scan_journal_namespace_with_admission<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        admit_row: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceFingerprint<'s>> {
        let contents =
            self.scan_journal_namespace_contents(namespace, budget, state, admit_row, visit)?;
        Ok(JournalNamespaceFingerprint::from_contents(self, contents))
    }

    pub(crate) fn scan_journal_namespace_contents<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        admit_row: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        self.scan_journal_namespace_core(
            namespace, budget, state, |_, _| Ok(()), admit_row, visit,
        )
    }

    pub(crate) fn scan_journal_namespace_preadmitted<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        preadmit: impl FnMut(&mut G, super::super::PhysicalReadAdmissionV1) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        self.scan_journal_namespace_core(namespace, budget, state, preadmit, |_| Ok(()), visit)
    }

    fn scan_journal_namespace_core<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        mut preadmit: impl FnMut(&mut G, super::super::PhysicalReadAdmissionV1) -> StoreResult<()>,
        mut admit_row: impl FnMut(&mut G) -> StoreResult<()>,
        mut visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        use super::super::PhysicalReadAdmissionV1;
        admit(budget)?;
        let prefix = namespace.physical_prefix()?;
        preadmit(state, PhysicalReadAdmissionV1::KeyProbe)?;
        let mut physical = self.snapshot.prefix(self.journal, &prefix);
        let mut scan = NamespaceScan::new(namespace, budget)?;
        for _ in 0..=budget.rows {
            preadmit(state, PhysicalReadAdmissionV1::KeyProbe)?;
            let Some(guard) = physical.next() else {
                return Ok(scan.finish());
            };
            admit_row(state)?;
            scan.charge_visit()?;
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            preadmit(state, PhysicalReadAdmissionV1::ValueSizeProbe)?;
            let length = self.snapshot.size_of(self.journal, &key)
                .map_err(|source| StoreError::Read { source })?
                .ok_or_else(|| namespace.refusal("namespace value size is missing"))?;
            let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
            scan.require_size(key.len(), length)?;
            preadmit(state, PhysicalReadAdmissionV1::Value { key_bytes: key.len(), value_bytes: length })?;
            let raw = self.snapshot.get(self.journal, &key)
                .map_err(|source| StoreError::Read { source })?
                .ok_or_else(|| namespace.refusal("namespace value is missing"))?;
            if raw.len() != length {
                return Err(namespace.refusal("namespace value size changed"));
            }
            scan.accept(&key, &raw)?;
            visit(state, &key, &raw)?;
        }
        Err(namespace.refusal("namespace scan did not prove exhaustion"))
    }

}

pub(crate) struct NamespaceScan {
    budget: ReadBudget,
    rows: u64,
    bytes: u64,
    digest: Sha256,
}

impl NamespaceScan {
    pub(crate) fn new(namespace: JournalNamespace<'_>, budget: ReadBudget) -> StoreResult<Self> {
        let mut digest = Sha256::new();
        digest.update(b"census-store-journal-namespace-v1\0");
        framed(&mut digest, namespace.phase().as_bytes())?;
        framed(&mut digest, namespace.logical_prefix().as_bytes())?;
        Ok(Self {
            budget,
            rows: 0,
            bytes: 0,
            digest,
        })
    }

    pub(crate) fn charge_visit(&mut self) -> StoreResult<()> {
        self.rows = charge(self.rows, 1, self.budget.rows, "journal physical rows")?;
        Ok(())
    }

    fn require_size(&self, key_bytes: usize, value_bytes: usize) -> StoreResult<()> {
        let key_limit = MAX_JOURNAL_KEY_BYTES.checked_add(5).ok_or(StoreError::CounterOverflow)?;
        size(key_bytes, key_limit)?;
        size(value_bytes, self.budget.record_bytes)?;
        let total = key_bytes.checked_add(value_bytes).ok_or(StoreError::CounterOverflow)?;
        charge(self.bytes, u64::try_from(total).map_err(|_| StoreError::CounterOverflow)?,
            self.budget.bytes, "journal encoded bytes")?;
        Ok(())
    }

    pub(crate) fn accept(&mut self, key: &[u8], raw: &[u8]) -> StoreResult<()> {
        let physical_key_limit = MAX_JOURNAL_KEY_BYTES
            .checked_add(5)
            .ok_or(StoreError::CounterOverflow)?;
        size(key.len(), physical_key_limit)?;
        size(raw.len(), self.budget.record_bytes)?;
        let bytes = key
            .len()
            .checked_add(raw.len())
            .ok_or(StoreError::CounterOverflow)?;
        let bytes = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
        self.bytes = charge(
            self.bytes,
            bytes,
            self.budget.bytes,
            "journal encoded bytes",
        )?;
        self.digest.update([1]);
        framed(&mut self.digest, key)?;
        let value_bytes = u64::try_from(raw.len()).map_err(|_| StoreError::CounterOverflow)?;
        self.digest.update(value_bytes.to_be_bytes());
        self.digest.update(Sha256::digest(raw));
        Ok(())
    }

    pub(crate) fn finish(mut self) -> JournalNamespaceContents {
        self.digest.update([255]);
        self.digest.update(self.rows.to_be_bytes());
        self.digest.update(self.bytes.to_be_bytes());
        JournalNamespaceContents {
            rows: self.rows,
            encoded_bytes: self.bytes,
            digest: self.digest.finalize().into(),
        }
    }
}

pub(crate) fn admit(budget: ReadBudget) -> StoreResult<()> {
    charge(0, budget.rows, MAX_NAMESPACE_ROWS, "journal admitted rows")?;
    charge(
        0,
        budget.bytes,
        MAX_NAMESPACE_BYTES,
        "journal admitted bytes",
    )?;
    size(budget.record_bytes, MAX_JOURNAL_VALUE_BYTES)
}

fn size(bytes: usize, max: usize) -> StoreResult<()> {
    if bytes > max {
        return Err(StoreError::RowTooLarge { bytes, max });
    }
    Ok(())
}

fn charge(current: u64, added: u64, available: u64, what: &'static str) -> StoreResult<u64> {
    let needed = current
        .checked_add(added)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        });
    }
    Ok(needed)
}

fn framed(digest: &mut Sha256, bytes: &[u8]) -> StoreResult<()> {
    let length = u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?;
    digest.update(length.to_be_bytes());
    digest.update(bytes);
    Ok(())
}
