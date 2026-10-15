use super::*;

pub(super) fn verify(store: &Store, reservation: &GraphReservation) -> StoreResult<()> {
    let mut visited = Vec::new();
    visited
        .try_reserve_exact(reservation.descriptors.len())
        .map_err(|_| invalid())?;
    visited.resize(reservation.descriptors.len(), false);
    crate::public_identity::paged_generation::capture::verify::verify(
        &store.snapshot_locked(),
        reservation.capture(),
        reservation.seal(),
        |node, object| {
            let index = reservation
                .descriptors
                .iter()
                .position(|entry| &entry.object == object)
                .ok_or_else(invalid)?;
            let seen = visited.get_mut(index).ok_or_else(invalid)?;
            if *seen {
                return Err(invalid());
            }
            let descriptor = reservation.descriptors.get(index).ok_or_else(invalid)?;
            let children = node.children()?;
            if children.len() != descriptor.children.len()
                || children
                    .iter()
                    .zip(&descriptor.children)
                    .any(|(actual, expected)| actual != &expected.0)
            {
                return Err(invalid());
            }
            *seen = true;
            Ok(())
        },
    )?;
    if visited.iter().any(|seen| !seen) {
        return Err(invalid());
    }
    Ok(())
}
