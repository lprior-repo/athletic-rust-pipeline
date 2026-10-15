use super::*;

pub(super) fn children_present(
    descriptors: &[Descriptor],
    children: &[ObjectRefV2],
) -> StoreResult<()> {
    for (index, child) in children.iter().enumerate() {
        if !descriptors.iter().any(|entry| &entry.object == child)
            || children
                .iter()
                .take(index)
                .any(|other| other.root() == child.root())
        {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate(
    reservation: &GraphReservation,
    identity: &census_domain::model::StoreIdentity,
) -> StoreResult<()> {
    if reservation.version != 3
        || bridge::capture_store(reservation.capture()) != identity
        || reservation.descriptors.is_empty()
        || reservation.descriptors.len() > MAX_DESCRIPTORS
    {
        return Err(invalid());
    }
    let mut rows = 0usize;
    for (index, entry) in reservation.descriptors.iter().enumerate() {
        let physical = &entry.physical;
        let ContextCharge { root, bytes, roots } = &entry.incremental;
        if entry.object.root().as_str() != physical.root()
            || root != physical.root()
            || entry.object.physical_bytes() != physical.full_bytes()
            || u64::from(entry.object.bytes()) != physical.jsonl_bytes()
            || u64::from(entry.object.records()) != physical.records()
            || !matches!((*roots, *bytes), (0, 0))
                && (*roots != 1 || *bytes != physical.full_bytes())
            || reservation
                .descriptors
                .iter()
                .take(index)
                .any(|prior| prior.object.root() == entry.object.root())
        {
            return Err(invalid());
        }
        rows = rows
            .checked_add(physical.row_count())
            .ok_or(StoreError::CounterOverflow)?;
        if rows > MAX_PHYSICAL_ROWS {
            return Err(invalid());
        }
        for (child_index, child) in entry.children.iter().enumerate() {
            if !reservation
                .descriptors
                .iter()
                .take(index)
                .any(|prior| prior.object == child.0)
                || entry
                    .children
                    .iter()
                    .take(child_index)
                    .any(|other| other.0.root() == child.0.root())
            {
                return Err(invalid());
            }
        }
    }
    closure(&reservation.descriptors, reservation.seal())
}

pub(super) fn closure(descriptors: &[Descriptor], seal: &ObjectRefV2) -> StoreResult<()> {
    if descriptors.is_empty()
        || descriptors.len() > MAX_DESCRIPTORS
        || descriptors.last().map(|entry| &entry.object) != Some(seal)
    {
        return Err(invalid());
    }
    let mut reachable = Vec::new();
    reachable
        .try_reserve_exact(descriptors.len())
        .map_err(|_| invalid())?;
    reachable.resize(descriptors.len(), false);
    *reachable.last_mut().ok_or_else(invalid)? = true;
    for (index, entry) in descriptors.iter().enumerate().rev() {
        if !reachable.get(index).copied().ok_or_else(invalid)? {
            continue;
        }
        for child in &entry.children {
            let child_index = descriptors
                .iter()
                .take(index)
                .position(|prior| prior.object == child.0)
                .ok_or_else(invalid)?;
            *reachable.get_mut(child_index).ok_or_else(invalid)? = true;
        }
    }
    if reachable.iter().any(|value| !value) {
        return Err(invalid());
    }
    Ok(())
}
