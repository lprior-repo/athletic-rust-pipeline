use super::super::{ledger::VerifiedLedger, material::Material, RetainedCaptureRootV1};
use super::*;

pub(super) fn collect<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<&'a RetainedCaptureRootV1>> {
    let mut roots: Vec<&RetainedCaptureRootV1> = Vec::new();
    for set in &ledger.introductions {
        for material in set {
            steps(grant, 1)?;
            if let Material::Retained { leaf } = material {
                insert(&mut roots, &leaf.root, grant)?;
            }
        }
    }
    Ok(roots)
}

fn insert<'a>(
    roots: &mut Vec<&'a RetainedCaptureRootV1>,
    root: &'a RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let mut start = 0usize;
    let mut end = roots.len();
    while start < end {
        let width = end.checked_sub(start).ok_or(StoreError::CounterOverflow)?;
        let middle = start
            .checked_add(width / 2)
            .ok_or(StoreError::CounterOverflow)?;
        let found = roots.get(middle).ok_or(StoreError::CounterOverflow)?;
        steps(grant, 1)?;
        match (found.schema.phase(), found.key.as_str())
            .cmp(&(root.schema.phase(), root.key.as_str()))
        {
            std::cmp::Ordering::Equal => {
                steps(grant, 1)?;
                if *found != root {
                    return Err(Error::Corrupt("frozen external root descriptor differs").into());
                }
                return Ok(());
            }
            std::cmp::Ordering::Less => {
                start = middle.checked_add(1).ok_or(StoreError::CounterOverflow)?
            }
            std::cmp::Ordering::Greater => end = middle,
        }
    }
    bound(
        "frozen archive external prefixes",
        roots
            .len()
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?,
        MAX_ROWS,
    )?;
    steps(
        grant,
        roots
            .len()
            .checked_sub(start)
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    roots
        .len()
        .checked_add(1)
        .and_then(|count| count.checked_mul(std::mem::size_of::<&RetainedCaptureRootV1>()))
        .ok_or(StoreError::CounterOverflow)?;
    material::allocate(roots)?;
    roots.insert(start, root);
    Ok(())
}

pub(super) fn require<P>(
    source: &FrozenRetainedCaptureSourceV1<'_>,
    ledger: &super::super::ledger::VerifiedLedger<P>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let roots = collect(ledger, grant)?;
    steps(grant, 1)?;
    let prefixes = source
        .archive
        .prefixes
        .get(2..)
        .ok_or(Error::Corrupt("frozen archive fixed prefixes"))?;
    if roots.len() != prefixes.len() {
        return Err(Error::Corrupt("frozen external closure differs").into());
    }
    for (expected, group) in roots.iter().zip(prefixes) {
        steps(grant, 1)?;
        let root = group
            .root
            .as_ref()
            .ok_or(Error::Corrupt("frozen archive prefix descriptor"))?;
        if root.schema != expected.schema
            || root.key != expected.key
            || root.payload.digest != expected.payload.digest.as_str()
            || root.payload.bytes != expected.payload.bytes
        {
            return Err(Error::Corrupt("frozen external closure differs").into());
        }
    }
    Ok(())
}
