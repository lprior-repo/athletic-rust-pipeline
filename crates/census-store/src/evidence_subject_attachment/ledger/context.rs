use super::*;

pub(super) fn find<'a>(
    sets: &'a [Vec<Material>],
    digest: &AttachmentDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<&'a Material>> {
    grant.ensure_available().map_err(Error::Domain)?;
    for set in sets {
        if let Some(material) = introduced_by_digest(set, digest, grant)? {
            return Ok(Some(material));
        }
    }
    Ok(None)
}

pub(super) fn accept(
    view: &StoreSnapshot<'_>,
    prior: &[Vec<Material>],
    introduced: &[Material],
    record: &ReviewedReferenceRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let references = record.command.reviewed_context.objects.as_slice();
    let mut previous = None;
    for material in introduced {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let reference = material.reference();
        if previous.is_some_and(|digest| digest >= &reference.digest)
            || find(prior, &reference.digest, grant)?.is_some()
        {
            return Err(Error::Corrupt("duplicate or unordered context introduction").into());
        }
        if !declared(references, reference, grant)? {
            return Err(Error::Corrupt("unused context introduction").into());
        }
        material.verify(view, grant)?;
        previous = Some(&reference.digest);
    }
    for reference in references {
        let material = match find(prior, &reference.digest, grant)? {
            Some(material) => material,
            None => introduced_by_digest(introduced, &reference.digest, grant)?
                .ok_or(Error::Corrupt("unresolved context reference"))?,
        };
        grant.charge_steps(1).map_err(Error::Domain)?;
        if material.reference() != reference {
            return Err(Error::Corrupt("redefined context reference").into());
        }
    }
    Ok(())
}

fn declared(
    references: &[AttachmentObjectReferenceV1],
    expected: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<bool> {
    Ok(ordered(references, expected, |reference| reference, grant)?.is_some())
}

fn introduced_by_digest<'a>(
    materials: &'a [Material],
    digest: &AttachmentDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<&'a Material>> {
    ordered(
        materials,
        digest,
        |material| &material.reference().digest,
        grant,
    )
}

fn ordered<'a, T, K: Ord>(
    rows: &'a [T],
    expected: &K,
    key: fn(&T) -> &K,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<&'a T>> {
    grant.ensure_available().map_err(Error::Domain)?;
    let (mut low, mut high) = (0usize, rows.len());
    while low < high {
        let width = high.checked_sub(low).ok_or(StoreError::CounterOverflow)?;
        let middle = low
            .checked_add(width / 2)
            .ok_or(StoreError::CounterOverflow)?;
        grant.charge_steps(1).map_err(Error::Domain)?;
        let row = rows
            .get(middle)
            .ok_or(Error::Corrupt("ordered context lookup bound"))?;
        match key(row).cmp(expected) {
            std::cmp::Ordering::Less => {
                low = middle.checked_add(1).ok_or(StoreError::CounterOverflow)?
            }
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return Ok(Some(row)),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
