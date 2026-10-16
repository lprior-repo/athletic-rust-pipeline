use super::super::fixture::*;
use crate::evidence_subject_attachment::{
    EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Receipt, Store, StoreError};
use census_domain::evidence_subject_attachment::*;

pub(super) enum Boundary {
    Preparation,
    Application,
}

pub(super) struct Committed {
    pub fixture: Fixture,
    pub context: Context,
    pub receipt: Receipt,
    pub record: ReviewedReferenceRecordV1,
}

pub(super) fn committed(label: &str) -> TestResult<Committed> {
    let fixture = Fixture::new()?;
    let context = Context::synthetic(label)?;
    let inspection = fixture.inspect()?;
    let command = context.command(label, &inspection)?;
    let receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let record = first_record(&fixture.store)?;
    Ok(Committed {
        fixture,
        context,
        receipt,
        record,
    })
}

pub(super) fn refuses_at_boundary(
    fixture: &Fixture,
    boundary: Boundary,
    mutate: impl FnOnce(&Store) -> TestResult,
    expected: &'static str,
) -> TestResult {
    let context = Context::synthetic("owner-reuse-following-unrelated-subject")?;
    let inspection = fixture.inspect()?;
    let command = context.command("owner-reuse-following-operation", &inspection)?;
    match boundary {
        Boundary::Preparation => {
            mutate(&fixture.store)?;
            let before = retained_state(&fixture.store)?;
            check!(
                matches!(fixture.store.prepare_evidence_subject_attachment_v1(
                command,
                Preparation::New { inspection, context: &context.borrowed() },
            ), Err(StoreError::EvidenceSubjectAttachment(AttachmentError::Corrupt(actual)))
                if actual == expected)
            );
            check!(eq; retained_state(&fixture.store)?, before);
        }
        Boundary::Application => {
            let prepared = prepare(&fixture.store, inspection, command, &context)?;
            let approval = prepared.approval_bytes().to_vec();
            let digest = prepared.approval_digest().clone();
            mutate(&fixture.store)?;
            let before = retained_state(&fixture.store)?;
            check!(matches!(fixture.store.apply_evidence_subject_attachment_v1(
                prepared, &approval, &digest,
            ), Err(StoreError::EvidenceSubjectAttachment(AttachmentError::Corrupt(actual)))
                if actual == expected));
            check!(eq; retained_state(&fixture.store)?, before);
        }
    }
    Ok(())
}

pub(super) fn replace_one_ascii(raw: &[u8], needle: &[u8]) -> TestResult<Vec<u8>> {
    check!(!needle.is_empty());
    let matches = raw
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == needle).then_some(index))
        .collect::<Vec<_>>();
    let [index] = matches.as_slice() else {
        return Err("corruption marker must occur exactly once".into());
    };
    let mut changed = raw.to_vec();
    let first = changed.get_mut(*index).ok_or("corruption offset absent")?;
    *first = if *first == b'0' { b'1' } else { b'0' };
    check!(eq; changed.len(), raw.len());
    check!(ne; changed.as_slice(), raw);
    Ok(changed)
}
