use super::super::super::issuance::ledger::Ledger;
use super::*;
use crate::RetainedCaptureRootV1;
pub(in super::super) fn roots(
    ledger: &Ledger,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<RetainedCaptureRootV1>> {
    let mut roots = Vec::new();
    ledger
        .records
        .iter()
        .zip(&ledger.contexts)
        .try_for_each(|(record, materials)| {
            [
                &record.command.source.result.result_root,
                &record.command.source.calendar.documentary_root,
            ]
            .into_iter()
            .try_for_each(|reference| {
                let bytes = super::super::super::source::object(materials, reference)?
                    .inline_bytes(grant)?;
                insert(&mut roots, codec::decode(&bytes, grant)?, grant)
            })?;
            materials.iter().try_for_each(|material| {
                if let super::super::super::source::Material::Retained { leaf } = material {
                    let bytes = codec::encode(&leaf.root, grant)?;
                    insert(&mut roots, codec::decode(&bytes, grant)?, grant)?;
                }
                Ok::<_, StoreError>(())
            })
        })?;
    roots.sort_unstable_by(|left, right| {
        (left.schema.phase(), &left.key).cmp(&(right.schema.phase(), &right.key))
    });
    Ok(roots)
}

fn insert(
    roots: &mut Vec<RetainedCaptureRootV1>,
    root: RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(roots.len()).map_err(grant_error)?;
    if let Some(prior) = roots
        .iter()
        .find(|prior| prior.schema == root.schema && prior.key == root.key)
    {
        if prior != &root {
            return Err(Error::Corrupt("source root disagreement").into());
        }
        return Ok(());
    }
    if roots.len()
        >= MAX_RESULT_CALENDAR_RECORDS
            .checked_mul(MAX_RESULT_CALENDAR_OBJECTS)
            .ok_or(Error::BudgetExceeded)?
    {
        return Err(Error::BudgetExceeded.into());
    }
    grant
        .charge_bytes(std::mem::size_of::<RetainedCaptureRootV1>())
        .map_err(grant_error)?;
    roots
        .try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    roots.push(root);
    Ok(())
}
