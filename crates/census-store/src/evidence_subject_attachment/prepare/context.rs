use super::*;
use census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1;

pub(super) fn introductions(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    supplied: &[AttachmentContextObjectV1<'_>],
    ledger: &Ledger<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<Material>> {
    if supplied.len() > 64 {
        return Err(Error::Corrupt("introduced context count").into());
    }
    let mut introduced = Vec::new();
    for issued in inspection.state.objects.iter().chain([
        &inspection.state.inputs.capture,
        &inspection.state.inputs.token,
    ]) {
        if let Some(existing) = ledger.find_object(&issued.reference().digest, grant)? {
            if existing.reference() != issued.reference() {
                return Err(Error::Corrupt("issued context reference redefined").into());
            }
        } else {
            let input = AttachmentContextObjectV1::Inline {
                reference: issued.reference(),
                bytes: issued.bytes(),
            };
            insert(&mut introduced, Material::introduce(&input, grant)?, grant)?;
        }
    }
    for input in supplied {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let reference = match input {
            AttachmentContextObjectV1::Inline { reference, .. } => *reference,
            AttachmentContextObjectV1::Retained(leaf) => &leaf.object,
        };
        if ledger.find_object(&reference.digest, grant)?.is_some() {
            return Err(Error::Corrupt("context supplier reintroduces retained object").into());
        }
        insert(&mut introduced, Material::introduce(input, grant)?, grant)?;
    }
    Ok(introduced)
}

fn insert(
    values: &mut Vec<Material>,
    value: Material,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if values.len() >= 64 {
        return Err(Error::Corrupt("introduced context count").into());
    }
    let mut index = values.len();
    for (position, existing) in values.iter().enumerate() {
        grant.charge_steps(1).map_err(Error::Domain)?;
        match existing.reference().digest.cmp(&value.reference().digest) {
            std::cmp::Ordering::Equal => {
                return Err(Error::Corrupt("duplicate context introduction").into())
            }
            std::cmp::Ordering::Greater => {
                index = position;
                break;
            }
            std::cmp::Ordering::Less => (),
        }
    }
    super::super::material::allocate(values)?;
    values.insert(index, value);
    Ok(())
}

pub(super) fn require_declared(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    expected: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if super::super::ledger::declared(command.reviewed_context.objects.as_slice(), expected, grant)?
    {
        return Ok(());
    }
    Err(Error::Corrupt("issued preparation input omitted from command").into())
}
