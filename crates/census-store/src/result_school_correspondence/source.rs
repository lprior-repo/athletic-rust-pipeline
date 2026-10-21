use super::*;
pub(super) use crate::evidence_subject_attachment::material::Material;
use crate::read::RetainedJournalReadSource;
use crate::{AttachmentContextObjectV1, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentSubjectRoleV1, RetainedDocumentSubjectReferenceV1,
    ReviewedReferenceRecordV1,
};

pub(super) mod checked;
mod person;
mod review;
mod statement;
mod target;
pub(super) use review::require_review;

pub(super) fn introduce(
    source: &ResultSchoolSourceClaimV1,
    context: &[AttachmentContextObjectV1<'_>],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<Material>> {
    if context.len() != source.evidence.len() || context.len() > MAX_RESULT_SCHOOL_OBJECTS {
        return Err(Error::SourceMismatch("context inventory").into());
    }
    grant
        .charge_bytes(
            context
                .len()
                .checked_mul(std::mem::size_of::<Material>())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let mut materials = Vec::new();
    materials
        .try_reserve_exact(context.len())
        .map_err(|_| Error::BudgetExceeded)?;
    context.iter().try_for_each(|input| {
        let material = match input {
            AttachmentContextObjectV1::Inline { .. } => Material::introduce(input, grant)?,
            AttachmentContextObjectV1::Retained(leaf) => {
                let bytes = codec::encode(*leaf, grant)?;
                Material::Retained {
                    leaf: codec::decode(&bytes, grant)?,
                }
            }
        };
        materials.push(material);
        Ok::<_, StoreError>(())
    })?;
    validate_inventory(source, &materials, grant)?;
    Ok(materials)
}

pub(super) fn require_current(
    view: &StoreSnapshot<'_>,
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    current: &crate::CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if current.inventory() != &source.person_inventory {
        return Err(Error::SourceMismatch("person inventory").into());
    }
    let subjects = current.discover_active_result_subjects_v1(grant)?;
    let subject = subjects
        .subjects()
        .iter()
        .find(|subject| {
            subject.record().command.operation == source.person_attachment.operation
                && subject.record().record_digest == source.person_attachment.digest
                && subject.record().command.target == source.person
                && subject.subject().locator == source.statement.row_locator
                && subject.subject().interpretation == source.statement.interpretation.digest
        })
        .ok_or(Error::SourceMismatch("active person attachment"))?;
    require_subject(view, source, materials, subject.subject(), grant)?;
    target::require_installed(view, source, materials, grant)
}

pub(super) fn require_historical(
    view: &impl RetainedJournalReadSource,
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    records: &[ReviewedReferenceRecordV1],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let record = person::original(source, records, grant)?;
    let subject = record
        .command
        .subjects
        .as_slice()
        .iter()
        .find(|subject| {
            subject.role == AttachmentSubjectRoleV1::ResultRow
                && subject.locator == source.statement.row_locator
                && subject.interpretation == source.statement.interpretation.digest
        })
        .ok_or(Error::SourceMismatch("retained person row"))?;
    require_subject(view, source, materials, subject, grant)
}

fn require_subject(
    view: &impl RetainedJournalReadSource,
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    materials
        .iter()
        .filter(|material| material.reference() != &source.statement.body)
        .try_for_each(|material| material.verify(view, grant))?;
    let result = root(
        materials,
        &source.statement.result_root,
        crate::RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
        grant,
    )?;
    if result.payload.digest != subject.capture || source.statement.body.digest != subject.body {
        return Err(Error::SourceMismatch("person capture/body").into());
    }
    crate::evidence_subject_attachment::external::with_checked(
        view,
        &result,
        grant,
        |_, root, grant| {
            let crate::evidence_subject_attachment::external::CheckedRoot::Html(capture) = root
            else {
                return Err(Error::SourceMismatch("result capture schema").into());
            };
            statement::require(
                source,
                materials,
                subject.namespace.as_str(),
                &result,
                &capture,
                grant,
            )
        },
    )?;
    let provider = school_root(materials, &source.target.provider_school_root, grant)?;
    crate::evidence_subject_attachment::external::with_checked(
        view,
        &provider,
        grant,
        |_, root, grant| {
            let crate::evidence_subject_attachment::external::CheckedRoot::School(capture) = root
            else {
                return Err(Error::SourceMismatch("school capture schema").into());
            };
            target::require_contracts(source, materials, &provider, &capture, grant)
        },
    )
}

pub(super) fn validate_inventory(
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let evidence = source.evidence.as_slice();
    if evidence.is_empty()
        || evidence.len() > MAX_RESULT_SCHOOL_OBJECTS
        || materials.len() != evidence.len()
    {
        return Err(Error::SourceMismatch("context count").into());
    }
    grant
        .charge_steps(
            evidence
                .len()
                .checked_mul(evidence.len())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    if evidence.windows(2).any(|pair| match pair {
        [left, right] => left.digest >= right.digest,
        _ => true,
    }) {
        return Err(Error::SourceMismatch("context order/duplicates").into());
    }
    evidence.iter().try_for_each(|reference| {
        if materials
            .iter()
            .filter(|material| material.reference() == reference)
            .count()
            != 1
        {
            return Err(Error::SourceMismatch("missing/duplicate context object").into());
        }
        Ok::<_, StoreError>(())
    })?;
    [
        &source.statement.result_root,
        &source.statement.body,
        &source.statement.interpretation,
        &source.statement.parser_contract,
        &source.target.provider_school_root,
        &source.target.physical_school_inventory,
        &source.target.year_contract,
    ]
    .into_iter()
    .try_for_each(|reference| object(materials, reference).map(|_| ()))
}

pub(super) fn object<'a>(
    materials: &'a [Material],
    reference: &AttachmentObjectReferenceV1,
) -> StoreResult<&'a Material> {
    materials
        .iter()
        .find(|material| material.reference() == reference)
        .ok_or_else(|| Error::SourceMismatch("undeclared evidence object").into())
}

pub(super) fn root(
    materials: &[Material],
    reference: &AttachmentObjectReferenceV1,
    schema: crate::RetainedCaptureRootSchemaV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<crate::RetainedCaptureRootV1> {
    let bytes = object(materials, reference)?.inline_bytes(grant)?;
    let root: crate::RetainedCaptureRootV1 = codec::decode(&bytes, grant)?;
    if root.schema != schema {
        return Err(Error::SourceMismatch("capture root schema").into());
    }
    Ok(root)
}

pub(super) fn school_root(
    materials: &[Material],
    reference: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<crate::RetainedCaptureRootV1> {
    let bytes = object(materials, reference)?.inline_bytes(grant)?;
    let root: crate::RetainedCaptureRootV1 = codec::decode(&bytes, grant)?;
    if !matches!(
        root.schema,
        crate::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1
            | crate::RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1
    ) {
        return Err(Error::SourceMismatch("capture root schema").into());
    }
    Ok(root)
}

mod selected;
pub(super) use selected::{physical_school, provider_school_root};

pub(super) fn require_accepted_head(
    head: &census_domain::evidence_subject_attachment::AttachmentPublicHeadV1,
    accepted: &crate::AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant
        .charge_steps(
            accepted
                .history_records()
                .len()
                .checked_add(2)
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let current =
        accepted.head().sequence == head.sequence && accepted.head().digest == head.digest.as_str();
    let historical = accepted
        .history_records()
        .iter()
        .any(|record| record.sequence == head.sequence && record.digest == head.digest.as_str());
    let genesis = accepted.history_records().first().is_some_and(|record| {
        record.previous.sequence == head.sequence && record.previous.digest == head.digest.as_str()
    });
    if !current && !historical && !genesis {
        return Err(Error::SourceMismatch("original accepted identity head").into());
    }
    Ok(())
}
