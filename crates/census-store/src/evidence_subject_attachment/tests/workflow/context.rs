use super::fixture::*;
use crate::evidence_subject_attachment::{
    EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::StoreError;

#[test]
fn declared_object_without_new_or_retained_material_cannot_prepare() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("missing-material")?;
    let inspection = fixture.inspect()?;
    let command = context.command("missing-material", &inspection)?;
    let supplied = context
        .objects
        .iter()
        .skip(1)
        .map(InlineObject::borrowed)
        .collect::<Vec<_>>();
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection,
                context: &supplied
            }
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("unresolved context reference")
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn undeclared_extra_material_is_not_silently_discarded_during_preparation() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("unused-material")?;
    let inspection = fixture.inspect()?;
    let command = context.command("unused-material", &inspection)?;
    let unused = InlineObject::synthetic("unused-material/not-in-reviewed-context")?;
    let mut supplied = context.borrowed();
    supplied.push(unused.borrowed());
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection,
                context: &supplied
            }
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("unused context introduction")
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn repeated_material_introduction_cannot_inflate_or_ambiguate_the_context_inventory() -> TestResult
{
    let fixture = Fixture::new()?;
    let context = Context::synthetic("duplicate-material")?;
    let inspection = fixture.inspect()?;
    let command = context.command("duplicate-material", &inspection)?;
    let duplicate = context
        .objects
        .first()
        .ok_or("controlled material absent")?;
    let mut supplied = context.borrowed();
    supplied.push(duplicate.borrowed());
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection,
                context: &supplied
            }
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("duplicate context introduction")
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn omitting_an_inspection_issued_full_input_token_refuses_even_with_complete_subject_facts(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("omitted-issued-token")?;
    let inspection = fixture.inspect()?;
    let omitted = inspection
        .issued_objects()
        .last()
        .ok_or("issued token absent")?
        .clone();
    let mut command = context.command("omitted-issued-token", &inspection)?;
    check!(ne; omitted.digest, command.reviewed_context.accepted_target_witness);
    let objects = command
        .reviewed_context
        .objects
        .as_slice()
        .iter()
        .filter(|object| *object != &omitted)
        .cloned()
        .collect();
    command.reviewed_context.objects = list(objects)?;
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection,
                context: &context.borrowed()
            }
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("issued preparation input omitted from command")
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
