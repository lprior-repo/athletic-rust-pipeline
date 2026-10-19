use super::*;
use crate::read::{
    admit_namespace_budget, retained_source_sealed, JournalNamespace, JournalNamespaceContents,
    NamespaceScan, ReadCounts, RetainedJournalBytes, RetainedJournalReadSource,
};
use crate::{PhysicalReadAdmissionV1, ReadBudget, RetainedCaptureRootV1};

pub(in crate::result_calendar_correspondence) struct Root {
    pub(in crate::result_calendar_correspondence) descriptor: RetainedCaptureRootV1,
    pub(in crate::result_calendar_correspondence) rows: Vec<(Vec<u8>, fjall::Slice)>,
}

pub(in crate::result_calendar_correspondence) fn retain(
    view: &StoreSnapshot<'_>,
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    attachments: &crate::CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    roots: &mut Vec<Root>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for (reference, schema) in [
        (
            &source.result.result_root,
            crate::RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
        ),
        (
            &source.calendar.documentary_root,
            crate::RetainedCaptureRootSchemaV1::ParticipationPacketV1,
        ),
    ] {
        insert(
            view,
            super::root(materials, reference, schema, grant)?,
            attachments,
            roots,
            grant,
        )?;
    }
    for material in materials {
        if let Material::Retained { leaf } = material {
            let bytes = codec::encode(&leaf.root, grant)?;
            insert(
                view,
                codec::decode(&bytes, grant)?,
                attachments,
                roots,
                grant,
            )?;
        }
    }
    Ok(())
}

fn insert(
    view: &StoreSnapshot<'_>,
    root: RetainedCaptureRootV1,
    attachments: &crate::CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    roots: &mut Vec<Root>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if attachments.contains_retained_capture_root(&root, grant)? {
        return Ok(());
    }
    grant.charge_steps(roots.len()).map_err(grant_error)?;
    if let Some(prior) = roots
        .iter()
        .find(|prior| prior.descriptor.schema == root.schema && prior.descriptor.key == root.key)
    {
        if prior.descriptor != root {
            return Err(Error::Corrupt("checked source root disagreement").into());
        }
        return Ok(());
    }
    let maximum = MAX_RESULT_CALENDAR_RECORDS
        .checked_mul(MAX_RESULT_CALENDAR_OBJECTS)
        .ok_or(Error::BudgetExceeded)?;
    if roots.len() >= maximum {
        return Err(Error::BudgetExceeded.into());
    }
    grant
        .charge_bytes(std::mem::size_of::<Root>())
        .map_err(grant_error)?;
    roots
        .try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    roots.push(capture(view, root, grant)?);
    Ok(())
}

fn capture(
    view: &StoreSnapshot<'_>,
    root: RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Root> {
    let mut state = (grant, Vec::new());
    view.scan_journal_namespace_preadmitted(
        crate::read::JournalNamespace::new(root.schema.phase(), &root.key)?,
        ReadBudget {
            rows: 6,
            bytes: 4 * 1024 * 1024,
            record_bytes: crate::MAX_JOURNAL_VALUE_BYTES,
        },
        &mut state,
        admit,
        |state, key, raw| retain_row(view, state, key, raw),
    )?;
    Ok(Root {
        descriptor: root,
        rows: state.1,
    })
}

fn admit(
    (grant, _): &mut (&mut AttachmentReplayGrantV1, Vec<(Vec<u8>, fjall::Slice)>),
    event: PhysicalReadAdmissionV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(grant_error)?;
    if let PhysicalReadAdmissionV1::Value {
        key_bytes,
        value_bytes,
    } = event
    {
        grant
            .charge_bytes(
                key_bytes
                    .checked_add(value_bytes)
                    .ok_or(Error::BudgetExceeded)?,
            )
            .map_err(grant_error)?;
    }
    Ok(())
}

fn retain_row(
    view: &StoreSnapshot<'_>,
    (grant, rows): &mut (&mut AttachmentReplayGrantV1, Vec<(Vec<u8>, fjall::Slice)>),
    key: &[u8],
    raw: &[u8],
) -> StoreResult<()> {
    grant.charge_bytes(key.len()).map_err(grant_error)?;
    let mut owned_key = Vec::new();
    owned_key
        .try_reserve_exact(key.len())
        .map_err(|_| Error::BudgetExceeded)?;
    owned_key.extend_from_slice(key);
    let bytes = match view.journal_get(key)? {
        Some(RetainedJournalBytes::Live(bytes)) if bytes.as_ref() == raw => bytes,
        Some(_) | None => return Err(Error::Corrupt("checked physical source changed").into()),
    };
    grant
        .charge_bytes(std::mem::size_of::<(Vec<u8>, fjall::Slice)>())
        .map_err(grant_error)?;
    rows.try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    rows.push((owned_key, bytes));
    Ok(())
}

pub(in crate::result_calendar_correspondence) struct Source<'a, 's> {
    view: &'a StoreSnapshot<'s>,
    roots: &'a [Root],
}

impl<'a, 's> Source<'a, 's> {
    pub(in crate::result_calendar_correspondence) fn new(
        view: &'a StoreSnapshot<'s>,
        roots: &'a [Root],
    ) -> Self {
        Self { view, roots }
    }

    fn lookup(&self, key: &[u8]) -> Option<&[u8]> {
        self.roots
            .iter()
            .flat_map(|root| &root.rows)
            .find(|(held, _)| held.as_slice() == key)
            .map(|(_, value)| value.as_ref())
    }

    fn owns_key(&self, key: &[u8]) -> bool {
        self.roots.iter().any(|root| {
            key.strip_prefix(root.descriptor.schema.phase().as_bytes())
                .and_then(|tail| tail.strip_prefix(&[0]))
                .is_some_and(|tail| tail.starts_with(root.descriptor.key.as_bytes()))
        })
    }
}

impl retained_source_sealed::Sealed for Source<'_, '_> {}

impl RetainedJournalReadSource for Source<'_, '_> {
    fn journal_size(&self, key: &[u8]) -> StoreResult<Option<u32>> {
        match self.lookup(key) {
            Some(raw) => Ok(Some(
                u32::try_from(raw.len()).map_err(|_| Error::BudgetExceeded)?,
            )),
            None if self.owns_key(key) => Ok(None),
            None => RetainedJournalReadSource::journal_size(self.view, key),
        }
    }

    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<RetainedJournalBytes<'_>>> {
        match self.lookup(key) {
            Some(raw) => Ok(Some(RetainedJournalBytes::Frozen(raw))),
            None if self.owns_key(key) => Ok(None),
            None => RetainedJournalReadSource::journal_get(self.view, key),
        }
    }

    fn scan_journal_namespace<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        mut admit: impl FnMut(&mut G) -> StoreResult<()>,
        mut visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        let held = self.roots.iter().find(|root| {
            root.descriptor.schema.phase() == namespace.phase()
                && root.descriptor.key == namespace.logical_prefix()
        });
        let Some(root) = held else {
            return RetainedJournalReadSource::scan_journal_namespace(
                self.view, namespace, budget, state, admit, visit,
            );
        };
        admit_namespace_budget(budget)?;
        let mut scan = NamespaceScan::new(namespace, budget)?;
        for (key, raw) in &root.rows {
            admit(state)?;
            scan.charge_visit()?;
            scan.accept(key, raw.as_ref())?;
            visit(state, key, raw.as_ref())?;
        }
        Ok(scan.finish())
    }

    fn visit_receipt_prefix<G>(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        RetainedJournalReadSource::visit_receipt_prefix(
            self.view, prefix, budget, state, admit, visit,
        )
    }
}
