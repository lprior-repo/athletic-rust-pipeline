use super::*;
use census_store::read::{JournalNamespace, JournalNamespaceFingerprint};

pub(super) struct Held<'s> {
    physical: Vec<(String, String, JournalNamespaceFingerprint<'s>)>,
    logical: Logical,
}

#[derive(Debug, PartialEq)]
pub(super) struct Logical {
    protected: ProtectedState,
    payloads: Vec<(String, String, Vec<u8>)>,
    receipts: Vec<Receipt>,
    physical_inventory: Value,
}

impl<'s> Held<'s> {
    pub(super) fn capture(store: &'s Store, sources: &Sources) -> TestResult<Self> {
        let snapshot = store.snapshot();
        let physical = namespaces(sources)
            .into_iter()
            .map(|(phase, key)| {
                let fingerprint = snapshot.journal_namespace_fingerprint(
                    JournalNamespace::new(&phase, &key)?,
                    school_budget(),
                )?;
                Ok((phase, key, fingerprint))
            })
            .collect::<TestResult<Vec<_>>>()?;
        Ok(Self {
            physical,
            logical: Logical::capture(store, sources)?,
        })
    }

    pub(super) fn require_unchanged(&self, store: &Store, sources: &Sources) -> TestResult {
        let snapshot = store.snapshot();
        for (phase, key, before) in &self.physical {
            let after = snapshot.journal_namespace_fingerprint(
                JournalNamespace::new(phase, key)?,
                school_budget(),
            )?;
            check!(before.same_contents(store, &after)?);
        }
        check!(eq; Logical::capture(store, sources)?, self.logical);
        self.logical.require_receipts(store)
    }
}

impl Logical {
    pub(super) fn capture(store: &Store, sources: &Sources) -> TestResult<Self> {
        let snapshot = store.snapshot();
        let mut payloads = Vec::new();
        for (phase, prefix) in namespaces(sources) {
            let mut keys = Vec::new();
            snapshot.for_each_journal_root_bounded(
                JournalNamespace::new(&phase, &prefix)?,
                school_budget(),
                |key| {
                    keys.push(key.to_owned());
                    Ok(())
                },
            )?;
            keys.sort();
            for key in keys {
                let raw = snapshot
                    .journal_payload_raw_bounded(
                        &phase,
                        &key,
                        JournalReadBudget::new(4 * 1024 * 1024)?,
                    )?
                    .ok_or("complete exact retained logical payload")?;
                payloads.push((phase.clone(), key, raw.get().as_bytes().to_vec()));
            }
        }
        payloads.sort();
        let mut receipts = Vec::new();
        let count = store.for_each_receipt(64, |receipt| {
            receipts.push(receipt);
            Ok(())
        })?;
        check!(eq; count, receipts.len());
        receipts.sort_by(|left, right| left.operation.cmp(&right.operation));
        Ok(Self {
            protected: protected_state(store)?,
            payloads,
            receipts,
            physical_inventory: report::physical_inventory(store, sources)?,
        })
    }

    pub(super) fn require_receipts(&self, store: &Store) -> TestResult {
        check!(eq; store.receipt_count()?, u64::try_from(self.receipts.len())?);
        for receipt in &self.receipts {
            check!(eq; store.receipt(&receipt.operation)?, Some(receipt.clone()));
        }
        Ok(())
    }
}

fn namespaces(sources: &Sources) -> Vec<(String, String)> {
    let mut namespaces = sources
        .roots
        .iter()
        .map(|root| (root.schema.phase().to_owned(), root.key.clone()))
        .collect::<Vec<_>>();
    namespaces.push(("evidence_subject".into(), "attachment/v1/record/".into()));
    namespaces
}
